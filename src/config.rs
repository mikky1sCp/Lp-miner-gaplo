use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub wallet: WalletConfig,
    pub contract: ContractConfig,
    pub miner: MinerSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub rpc_urls: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WalletConfig {
    pub seed_phrase: Option<String>,
    pub private_key: Option<String>,
    pub wallet_address: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ContractConfig {
    pub contract_address: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MinerSettings {
    pub max_wallets: usize,
    pub gas_thresholds: f64,
    pub token_withdrawal_multiplier: f64,
    pub log_level: u8,
    pub mining_threads_per_wallet: usize,
    pub blocks_to_wait: u64,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
}