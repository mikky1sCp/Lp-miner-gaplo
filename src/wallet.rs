use ethers::signers::{LocalWallet, MnemonicBuilder, Signer}; // добавили Signer
use ethers::signers::coins_bip39::English;
use serde::{Deserialize, Serialize};
use std::fs;
use crate::error::Result;
use rand::thread_rng;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WalletEntry {
    pub address: String,
    pub private_key: String,
    pub index: u32,
}

pub fn load_wallets(path: &str) -> Result<Vec<WalletEntry>> {
    if !std::path::Path::new(path).exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path)?;
    let wallets: Vec<WalletEntry> = serde_json::from_str(&contents)?;
    Ok(wallets)
}

pub fn save_wallets(path: &str, wallets: &[WalletEntry]) -> Result<()> {
    let json = serde_json::to_string_pretty(wallets)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn create_new_wallet(seed_phrase: Option<&str>, current_index: u32) -> Result<WalletEntry> {
    let wallet = if let Some(phrase) = seed_phrase {
        let wallet: LocalWallet = MnemonicBuilder::<English>::default()
            .phrase(phrase)
            .index(current_index)?
            .build()?;
        wallet
    } else {
        LocalWallet::new(&mut thread_rng())
    };
    Ok(WalletEntry {
        address: hex::encode(wallet.address().as_bytes()),
        private_key: hex::encode(wallet.signer().to_bytes()),
        index: current_index,
    })
}