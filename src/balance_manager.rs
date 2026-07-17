use crate::error::{MinerError, Result};
use crate::rpc::RpcPool;
use crate::metrics::Metrics;
use ethers::prelude::*;
use ethers::providers::{Provider, Http, Middleware};
use ethers::signers::LocalWallet;
use ethers::core::types::transaction::eip1559::Eip1559TransactionRequest;
use ethers::core::types::{BlockNumber, TransactionRequest};
use std::sync::Arc;
use tracing::{info, warn, error};
use std::sync::atomic::AtomicU64;

pub type Client = Provider<Http>;
pub type SignerClient = SignerMiddleware<Client, LocalWallet>;
pub type ContractInstance = ethers::contract::Contract<SignerClient>;

pub struct BalanceManager {
    main_address: Address,
    main_client: Arc<SignerClient>,
    gas_threshold: U256,
    withdrawal_multiplier: f64,
    commission_address: Option<Address>,
    commission_multiplier: f64,
    contract: Arc<ContractInstance>,
    rpc_pool: Arc<RpcPool>,
    metrics: Arc<Metrics>,
    main_nonce: AtomicU64,
}

impl BalanceManager {
    pub fn new(
        main_address: Address,
        main_client: Arc<SignerClient>,
        gas_threshold: f64,
        withdrawal_multiplier: f64,
        commission_address: Option<Address>,
        commission_multiplier: f64,
        contract: Arc<ContractInstance>,
        rpc_pool: Arc<RpcPool>,
        metrics: Arc<Metrics>,
    ) -> Self {
        let threshold_wei = U256::from((gas_threshold * 10_f64.powi(18)) as u64);
        Self {
            main_address,
            main_client,
            gas_threshold: threshold_wei,
            withdrawal_multiplier,
            commission_address,
            commission_multiplier,
            contract,
            rpc_pool,
            metrics,
            main_nonce: AtomicU64::new(0),
        }
    }

    pub async fn ensure_balance(&self, wallet_address: Address, _wallet_client: Arc<SignerClient>) -> Result<bool> {
        self.metrics.inc_balance_checks();
        let balance = self.rpc_pool.get_balance(wallet_address, None).await?;
        if balance < self.gas_threshold {
            info!(
                "Wallet {} balance {} is below threshold {}, topping up",
                wallet_address, balance, self.gas_threshold
            );
            let amount_wei = self.gas_threshold * U256::from((self.withdrawal_multiplier * 100.0) as u64) / U256::from(100u64);
            if amount_wei == U256::zero() {
                return Ok(false);
            }
            let receipt = self.transfer_gas(self.main_client.clone(), wallet_address, amount_wei).await?;
            if receipt.status == Some(U64::one()) {
                info!("Successfully transferred {} GAPLO to {}", self.wei_to_ether(amount_wei), wallet_address);
                self.metrics.inc_balance_topups();
                Ok(true)
            } else {
                error!("Transfer to {} reverted", wallet_address);
                self.metrics.inc_failed_transactions();
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    async fn transfer_gas(
        &self,
        client: Arc<SignerClient>,
        to: Address,
        amount: U256,
    ) -> Result<TransactionReceipt> {
        let calldata = self.contract
            .method::<(Address, U256), bool>("transfer", (to, amount))
            .map_err(|e| MinerError::Abi(e.to_string()))?
            .calldata()
            .ok_or_else(|| MinerError::Other("calldata missing".into()))?;
        let gas_estimate = self.contract
            .method::<(Address, U256), bool>("transfer", (to, amount))
            .map_err(|e| MinerError::Abi(e.to_string()))?
            .estimate_gas()
            .await?;
        let fee_history = client.fee_history(1, BlockNumber::Latest, &[])
            .await
            .map_err(|e| MinerError::Other(e.to_string()))?;
        let base_fee = fee_history.base_fee_per_gas.last().unwrap_or(&U256::zero()).clone();
        let max_priority_fee = U256::from(2_000_000_000u64);
        let max_fee = base_fee + max_priority_fee;
        let chain_id = client.get_chainid()
            .await
            .map_err(|e| MinerError::Other(e.to_string()))?
            .as_u64();

        let tx = Eip1559TransactionRequest::new()
            .to(self.contract.address())
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

    pub async fn send_commissions(
        &self,
        wallet_address: Address,
        wallet_client: Arc<SignerClient>,
        total_mined: U256,
    ) -> Result<()> {
        if total_mined < U256::from(20) {
            return Ok(());
        }
        let amount_to_withdraw = self.gas_threshold * U256::from((self.withdrawal_multiplier * 100.0) as u64) / U256::from(100u64);
        if amount_to_withdraw == U256::zero() {
            return Ok(());
        }
        let balance = self.rpc_pool.get_balance(wallet_address, None).await?;
        if balance < amount_to_withdraw {
            warn!("Insufficient balance on {} to pay commissions", wallet_address);
            return Ok(());
        }

        let receipt1 = self.transfer_gas(wallet_client.clone(), self.main_address, amount_to_withdraw).await?;
        if receipt1.status != Some(U64::one()) {
            error!("Commission transfer to main wallet from {} reverted", wallet_address);
            return Err(MinerError::Transaction("Commission transfer reverted".into()));
        }
        info!("Sent {} GAPLO from {} to main wallet", self.wei_to_ether(amount_to_withdraw), wallet_address);

        if let Some(comm_addr) = self.commission_address {
            let comm_amount = amount_to_withdraw * U256::from((self.commission_multiplier * 100.0) as u64) / U256::from(100u64);
            if comm_amount > U256::zero() && balance >= amount_to_withdraw + comm_amount {
                let receipt2 = self.transfer_gas(wallet_client.clone(), comm_addr, comm_amount).await?;
                if receipt2.status == Some(U64::one()) {
                    info!("Sent {} GAPLO from {} to commission address", self.wei_to_ether(comm_amount), wallet_address);
                } else {
                    warn!("Commission transfer to {} from {} reverted", comm_addr, wallet_address);
                }
            }
        }
        Ok(())
    }

    fn wei_to_ether(&self, wei: U256) -> f64 {
        let divisor = U256::from(10u64.pow(18));
        let eth = wei / divisor;
        let remainder = wei % divisor;
        let frac = remainder.as_u64() as f64 / 10_f64.powi(18);
        eth.as_u64() as f64 + frac
    }
}