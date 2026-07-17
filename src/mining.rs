use crate::error::{MinerError, Result};
use crate::rpc::RpcPool;
use crate::crypto::{hash_nonce, generate_nonce};
use crate::metrics::Metrics;
use ethers::prelude::*;
use ethers::providers::{Provider, Http, Middleware};
use ethers::signers::LocalWallet;
use ethers::core::types::transaction::eip1559::Eip1559TransactionRequest;
use ethers::core::types::{BlockNumber, TransactionRequest};
use rayon::prelude::*;
use std::sync::Arc;
use tracing::{info, error, warn};
use tokio::time::{sleep, Duration};

pub type Client = Provider<Http>;
pub type SignerClient = SignerMiddleware<Client, LocalWallet>;
pub type ContractInstance = ethers::contract::Contract<SignerClient>;

pub struct MinerParams {
    pub last_block: U256,
    pub current_difficulty: U256,
    pub total_mined: U256,
    pub prev_hash: U256,
}

pub async fn get_miner_params(
    contract: &ContractInstance,
    wallet: Address,
) -> Result<MinerParams> {
    let (last_block, difficulty, total_mined, prev_hash): (U256, U256, U256, U256) = contract
        .method("miner_params", wallet)
        .map_err(|e| MinerError::Abi(e.to_string()))?
        .call()
        .await?;
    let diff = if difficulty == U256::zero() {
        U256::from_dec_str("115792089237316195423570985008687907853269984665640564039457584007913129639935").unwrap()
    } else {
        difficulty
    };
    Ok(MinerParams {
        last_block,
        current_difficulty: diff,
        total_mined,
        prev_hash,
    })
}

pub fn find_nonce_parallel(
    wallet: Address,
    params: &MinerParams,
    threads: usize,
) -> U256 {
    let _ = wallet; // подавляем предупреждение (wallet используется в замыкании)
    let found = (0..threads)
        .into_par_iter()
        .find_map_any(|_| {
            loop {
                let nonce = generate_nonce();
                let hash = hash_nonce(
                    nonce,
                    wallet,
                    params.current_difficulty,
                    params.prev_hash,
                    params.total_mined,
                );
                if hash < params.current_difficulty {
                    return Some(nonce);
                }
            }
        })
        .expect("Should find nonce");
    found
}

pub async fn send_mine_transaction(
    contract: &ContractInstance,
    wallet: Address,
    nonce: U256,
    client: Arc<SignerClient>,
) -> Result<TransactionReceipt> {
    let mut nonce_bytes = [0u8; 32];
    nonce.to_big_endian(&mut nonce_bytes);
    let nonce_vec = nonce_bytes.to_vec();

    let gas_estimate = contract
        .method::<Vec<u8>, ()>("mine", nonce_vec.clone())
        .map_err(|e| MinerError::Abi(e.to_string()))?
        .estimate_gas()
        .await?;

    let fee_history = client.fee_history(1, BlockNumber::Latest, &[])
        .await
        .map_err(|e| MinerError::Other(e.to_string()))?;
    let base_fee = fee_history.base_fee_per_gas.last().unwrap_or(&U256::zero()).clone();
    let max_priority_fee = U256::from(2_000_000_000u64);
    let max_fee = base_fee + max_priority_fee;

    let calldata = contract
        .method::<Vec<u8>, ()>("mine", nonce_vec)
        .map_err(|e| MinerError::Abi(e.to_string()))?
        .calldata()
        .ok_or_else(|| MinerError::Other("calldata missing".into()))?;

    let chain_id = client.get_chainid()
        .await
        .map_err(|e| MinerError::Other(e.to_string()))?
        .as_u64();

    let tx = Eip1559TransactionRequest::new()
        .to(contract.address())
        .data(calldata)
        .gas(gas_estimate + 1000)
        .max_fee_per_gas(max_fee)
        .max_priority_fee_per_gas(max_priority_fee)
        .chain_id(chain_id);

    let pending = client.send_transaction::<TransactionRequest>(tx.into(), None)
        .await
        .map_err(|e| MinerError::Other(e.to_string()))?;
    let receipt = pending.confirmations(1).await?;
    receipt.ok_or_else(|| MinerError::Transaction("No receipt".into()))
}

pub async fn run_miner_for_wallet(
    contract: Arc<ContractInstance>,
    wallet: Address,
    client: Arc<SignerClient>,
    config: &crate::config::Config,
    rpc_pool: &Arc<RpcPool>,
    metrics: &Arc<Metrics>,
) -> Result<MinerParams> {
    let params = get_miner_params(&contract, wallet).await?;
    info!(
        "Wallet {}: difficulty {}, total mined {}",
        wallet, params.current_difficulty, params.total_mined
    );

    let balance = rpc_pool.get_balance(wallet, None).await?;
    let balance_ether = balance / U256::from(10u64.pow(18));
    if balance_ether < U256::from((config.miner.gas_thresholds * 10_f64.powi(18)) as u64) {
        warn!("Wallet {} has low balance: {} GAPLO", wallet, balance_ether);
    }

    let current_block = rpc_pool.get_block_number().await?;
    let last_block = params.last_block;
    let blocks_to_wait = config.miner.blocks_to_wait;
    if current_block < last_block + U256::from(blocks_to_wait) {
        let wait_blocks = last_block + U256::from(blocks_to_wait) - current_block;
        info!("Waiting {} blocks before mining", wait_blocks);
        loop {
            let new_block = rpc_pool.get_block_number().await?;
            if new_block >= last_block + U256::from(blocks_to_wait) {
                break;
            }
            sleep(Duration::from_secs(3)).await;
        }
    }

    let threads = config.miner.mining_threads_per_wallet;
    info!("Searching nonce for wallet {}", wallet);
    let nonce = find_nonce_parallel(wallet, &params, threads);
    info!("Found nonce: {}", nonce);

    let receipt = send_mine_transaction(&contract, wallet, nonce, client.clone()).await?;
    info!("Transaction sent, block: {}", receipt.block_number.unwrap_or_default());

    if receipt.status == Some(U64::one()) {
        info!("Transaction successful for wallet {}", wallet);
        metrics.inc_mined_blocks();
        let new_params = get_miner_params(&contract, wallet).await?;
        Ok(new_params)
    } else {
        error!("Transaction reverted for wallet {}", wallet);
        metrics.inc_failed_transactions();
        Err(MinerError::Transaction("Reverted".into()))
    }
}