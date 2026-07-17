mod config;
mod error;
mod rpc;
mod crypto;
mod mining;
mod wallet;
mod balance_manager;
mod metrics;

use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, error};
use tracing_subscriber;
use ethers::prelude::*;
use ethers::providers::{Provider, Http};
use ethers::signers::LocalWallet;
use crate::metrics::Metrics;

type Client = Provider<Http>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting Lp-miner-gaplo");

    let config = config::Config::from_file("config.toml")
        .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;
    info!("Config loaded");

    let metrics = Arc::new(Metrics::new());

    let rpc_pool = rpc::RpcPool::new(config.server.rpc_urls.clone()).await?;
    info!("RPC pool initialized with {} endpoints", config.server.rpc_urls.len());

    let abi = rpc::load_abi()?;
    let contract_addr: Address = config.contract.contract_address.parse()?;

    // Создаём основной клиент с подписью
    let main_private_key = config.wallet.private_key
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Main private key not set"))?
        .clone();
    let main_wallet: LocalWallet = main_private_key.parse()?;
    let (_, client) = rpc_pool.get_current_client().await;
    let main_client = Arc::new(SignerMiddleware::new(client, main_wallet));

    // Создаём контракт для основного клиента
    let contract = Arc::new(Contract::new(contract_addr, abi, main_client.clone()));

    let main_address = main_client.address();
    let comm_addr: Option<Address> = Some("0x3200eEaBa4a47D58794727B5A4a8D04673Ec6772".parse()?);

    let balance_manager = Arc::new(balance_manager::BalanceManager::new(
        main_address,
        main_client.clone(),
        config.miner.gas_thresholds,
        config.miner.token_withdrawal_multiplier,
        comm_addr,
        0.1,
        contract.clone(),
        rpc_pool.clone(),
        metrics.clone(),
    ));

    // Загружаем / создаём кошельки
    let mut wallets = wallet::load_wallets("wallets.json").unwrap_or_else(|_| Vec::new());
    if wallets.len() < config.miner.max_wallets {
        info!("Creating {} new wallets", config.miner.max_wallets - wallets.len());
        let seed = config.wallet.seed_phrase.as_deref();
        let mut next_index = wallets.last().map(|w| w.index + 1).unwrap_or(0);
        for _ in wallets.len()..config.miner.max_wallets {
            let new_wallet = wallet::create_new_wallet(seed, next_index)?;
            next_index += 1;
            wallets.push(new_wallet);
        }
        wallet::save_wallets("wallets.json", &wallets)?;
    }

    // Клонируем данные для передачи в задачи
    let wallet_entries: Vec<wallet::WalletEntry> = wallets.clone();
    for (idx, wallet_entry) in wallet_entries.into_iter().enumerate() {
        let wallet_addr: Address = wallet_entry.address.parse()?;
        let priv_key = wallet_entry.private_key.clone();

        // Создаём подписанный клиент для этого кошелька
        let wallet_signer: LocalWallet = priv_key.parse()?;
        let (_, provider) = rpc_pool.get_current_client().await;
        let wallet_client = Arc::new(SignerMiddleware::new(provider, wallet_signer));

        // Клонируем контракт с этим клиентом
        let contract_clone = Arc::new(Contract::new(
            contract.address(),
            contract.abi().clone(),
            wallet_client.clone()
        ));

        let bm = balance_manager.clone();
        let metrics_clone = metrics.clone();
        let config_clone = config.clone();
        let rpc_pool_clone = rpc_pool.clone();

        tokio::spawn(async move {
            info!("Starting miner thread {} for wallet {}", idx, wallet_entry.address);
            loop {
                // Проверяем баланс и пополняем при необходимости
                if let Err(e) = bm.ensure_balance(wallet_addr, wallet_client.clone()).await {
                    error!("Balance check failed for {}: {}", wallet_addr, e);
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }

                // Запускаем майнинг
                match mining::run_miner_for_wallet(
                    contract_clone.clone(),
                    wallet_addr,
                    wallet_client.clone(),
                    &config_clone,
                    &rpc_pool_clone,
                    &metrics_clone,
                ).await {
                    Ok(new_params) => {
                        if let Err(e) = bm.send_commissions(wallet_addr, wallet_client.clone(), new_params.total_mined).await {
                            error!("Commission send failed for {}: {}", wallet_addr, e);
                        }
                    }
                    Err(e) => {
                        error!("Miner error for wallet {}: {}", wallet_addr, e);
                        metrics_clone.inc_miner_errors();
                        sleep(Duration::from_secs(10)).await;
                    }
                }
                sleep(Duration::from_secs(1)).await;
            }
        });
    }

    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");
    metrics.print_summary();

    Ok(())
}