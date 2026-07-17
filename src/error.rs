use thiserror::Error;
use ethers::prelude::*;
use ethers::contract::AbiError;
use ethers::providers::ProviderError;
use ethers::signers::WalletError;
use ethers::utils::hex::FromHexError;

#[derive(Error, Debug)]
pub enum MinerError {
    #[error("RPC error: {0}")]
    Rpc(#[from] ProviderError),
    #[error("Contract error (Provider): {0}")]
    ContractProvider(#[from] ContractError<Provider<Http>>),
    #[error("Transaction error: {0}")]
    Transaction(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Wallet error: {0}")]
    Wallet(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("ABI error: {0}")]
    Abi(String),
    #[error("Hex parsing error: {0}")]
    Hex(#[from] FromHexError),
    #[error("Signer error: {0}")]
    Signer(String),
    #[error("Other: {0}")]
    Other(String),
}

// Конкретная реализация для контракта с подписанным клиентом
impl From<ContractError<SignerMiddleware<Provider<Http>, LocalWallet>>> for MinerError {
    fn from(e: ContractError<SignerMiddleware<Provider<Http>, LocalWallet>>) -> Self {
        MinerError::Other(e.to_string())
    }
}

impl From<ethers::abi::Error> for MinerError {
    fn from(e: ethers::abi::Error) -> Self {
        MinerError::Abi(e.to_string())
    }
}

impl From<AbiError> for MinerError {
    fn from(e: AbiError) -> Self {
        MinerError::Abi(e.to_string())
    }
}

impl From<WalletError> for MinerError {
    fn from(e: WalletError) -> Self {
        MinerError::Signer(e.to_string())
    }
}

impl From<&str> for MinerError {
    fn from(s: &str) -> Self {
        MinerError::Other(s.to_string())
    }
}

impl From<String> for MinerError {
    fn from(s: String) -> Self {
        MinerError::Other(s)
    }
}

pub type Result<T> = std::result::Result<T, MinerError>;