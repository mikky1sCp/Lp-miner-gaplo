use std::sync::atomic::{AtomicU64, Ordering};
use tracing::info;
use once_cell::sync::Lazy;

pub struct Metrics {
    mined_blocks: AtomicU64,
    failed_transactions: AtomicU64,
    rpc_switches: AtomicU64,
    balance_checks: AtomicU64,
    balance_topups: AtomicU64,
    miner_errors: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            mined_blocks: AtomicU64::new(0),
            failed_transactions: AtomicU64::new(0),
            rpc_switches: AtomicU64::new(0),
            balance_checks: AtomicU64::new(0),
            balance_topups: AtomicU64::new(0),
            miner_errors: AtomicU64::new(0),
        }
    }

    pub fn global() -> &'static Metrics {
        static INSTANCE: Lazy<Metrics> = Lazy::new(|| Metrics::new());
        &INSTANCE
    }

    pub fn inc_mined_blocks(&self) {
        self.mined_blocks.fetch_add(1, Ordering::SeqCst);
    }
    pub fn inc_failed_transactions(&self) {
        self.failed_transactions.fetch_add(1, Ordering::SeqCst);
    }
    pub fn inc_rpc_switches(&self) {
        self.rpc_switches.fetch_add(1, Ordering::SeqCst);
    }
    pub fn inc_balance_checks(&self) {
        self.balance_checks.fetch_add(1, Ordering::SeqCst);
    }
    pub fn inc_balance_topups(&self) {
        self.balance_topups.fetch_add(1, Ordering::SeqCst);
    }
    pub fn inc_miner_errors(&self) {
        self.miner_errors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn print_summary(&self) {
        info!("--- Metrics Summary ---");
        info!("Mined blocks: {}", self.mined_blocks.load(Ordering::SeqCst));
        info!("Failed transactions: {}", self.failed_transactions.load(Ordering::SeqCst));
        info!("RPC switches: {}", self.rpc_switches.load(Ordering::SeqCst));
        info!("Balance checks: {}", self.balance_checks.load(Ordering::SeqCst));
        info!("Balance topups: {}", self.balance_topups.load(Ordering::SeqCst));
        info!("Miner errors: {}", self.miner_errors.load(Ordering::SeqCst));
    }
}