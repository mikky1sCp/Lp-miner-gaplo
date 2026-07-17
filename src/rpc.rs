use crate::error::Result;
use ethers::prelude::*;
use ethers::providers::{Provider, Http};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{warn, error};
use crate::metrics::Metrics;

pub type Client = Provider<Http>;
pub type ContractInstance = ethers::contract::Contract<Client>;

pub struct RpcPool {
    urls: Vec<String>,
    current_index: RwLock<usize>,
    clients: Vec<Client>,
}

impl RpcPool {
    pub async fn new(urls: Vec<String>) -> Result<Arc<Self>> {
        if urls.is_empty() {
            return Err(crate::error::MinerError::Other("No RPC URLs provided".into()));
        }
        let clients: Result<Vec<Client>> = urls.iter()
            .map(|url| Client::try_from(url.as_str()).map_err(|e| crate::error::MinerError::Other(e.to_string())))
            .collect();
        let pool = Arc::new(Self {
            urls,
            current_index: RwLock::new(0),
            clients: clients?,
        });
        Ok(pool)
    }

    pub async fn get_current_client(&self) -> (usize, Client) {
        let idx = *self.current_index.read().await;
        (idx, self.clients[idx].clone())
    }

    async fn switch_to_next(&self) {
        let mut idx = self.current_index.write().await;
        *idx = (*idx + 1) % self.clients.len();
        let new_idx = *idx;
        warn!("Switched RPC endpoint to index {}: {}", new_idx, self.urls[new_idx]);
        Metrics::global().inc_rpc_switches();
    }

    pub async fn get_block_number(&self) -> Result<U256> {
        let mut attempts = 0;
        let max_attempts = self.clients.len() * 2 + 1;
        loop {
            let (idx, client) = self.get_current_client().await;
            match client.get_block_number().await {
                Ok(num) => return Ok(U256::from(num.as_u64())),
                Err(e) => {
                    error!("get_block_number failed on endpoint {}: {}", idx, e);
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(e.into());
                    }
                    self.switch_to_next().await;
                }
            }
        }
    }

    pub async fn get_balance(&self, address: Address, block: Option<BlockId>) -> Result<U256> {
        let mut attempts = 0;
        let max_attempts = self.clients.len() * 2 + 1;
        loop {
            let (idx, client) = self.get_current_client().await;
            match client.get_balance(address, block).await {
                Ok(bal) => return Ok(bal),
                Err(e) => {
                    error!("get_balance failed on endpoint {}: {}", idx, e);
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(e.into());
                    }
                    self.switch_to_next().await;
                }
            }
        }
    }

    pub async fn get_transaction_count(&self, address: Address, block: Option<BlockId>) -> Result<U256> {
        let mut attempts = 0;
        let max_attempts = self.clients.len() * 2 + 1;
        loop {
            let (idx, client) = self.get_current_client().await;
            match client.get_transaction_count(address, block).await {
                Ok(count) => return Ok(count),
                Err(e) => {
                    error!("get_transaction_count failed on endpoint {}: {}", idx, e);
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(e.into());
                    }
                    self.switch_to_next().await;
                }
            }
        }
    }

    pub async fn fee_history(&self, block_count: u64, last_block: BlockNumber, reward_percentiles: &[f64]) -> Result<FeeHistory> {
        let mut attempts = 0;
        let max_attempts = self.clients.len() * 2 + 1;
        loop {
            let (idx, client) = self.get_current_client().await;
            match client.fee_history(block_count, last_block, reward_percentiles).await {
                Ok(fh) => return Ok(fh),
                Err(e) => {
                    error!("fee_history failed on endpoint {}: {}", idx, e);
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(e.into());
                    }
                    self.switch_to_next().await;
                }
            }
        }
    }

    pub async fn get_chainid(&self) -> Result<U256> {
        let mut attempts = 0;
        let max_attempts = self.clients.len() * 2 + 1;
        loop {
            let (idx, client) = self.get_current_client().await;
            match client.get_chainid().await {
                Ok(id) => return Ok(id),
                Err(e) => {
                    error!("get_chainid failed on endpoint {}: {}", idx, e);
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(e.into());
                    }
                    self.switch_to_next().await;
                }
            }
        }
    }
}

pub fn load_abi() -> Result<ethers::abi::Abi> {
    let abi_str = std::fs::read_to_string("abi.json")?;
    let abi: ethers::abi::Abi = serde_json::from_str(&abi_str)?;
    Ok(abi)
}