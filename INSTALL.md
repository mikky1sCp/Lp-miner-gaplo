# Installing and Running Gaplo Miner (Rust)

This guide will help you run the miner on your computer, even if you have no programming experience.

---

## 🔧 Requirements

- **Operating System**: Windows, Linux, or macOS.
- **Internet**: Access to APLO network RPC nodes.
- **Wallet**: You need the address and private key (or seed phrase) of the main wallet that will fund the child wallets.

---

## 📦 Step 1. Installing Rust

Rust is the language used to write the miner. You only need to install it once.

1. Go to [https://rustup.rs](https://rustup.rs)
2. Download and run the `rustup-init.exe` installer (for Windows) or run the following command in your terminal (Linux/macOS):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
3.  During installation, select "Default installation" (simply press Enter).
4.  Restart the terminal (command prompt) after completion.
Check that Rust is installed:

```bash
cargo --version
```
A version should appear (e.g., cargo 1.82.0).

## 📂 Step 2. Downloading the miner
Clone the source code repository:

```bash
git clone https://github.com/your_username/gaplo-miner-rust.git
cd gaplo-miner-rust
```
If you do not have git, download the ZIP archive from GitHub, extract it to a folder of your choice, and then open a terminal in that folder.

## ⚙️ Step 3. Configuration
In the miner folder, create a `config.toml` file (if it doesn't exist, copy it from `config.toml.example`).

Open `config.toml` in Notepad and fill it in:
```toml
[server]
rpc_urls = [
    "https://pub1.aplocoin.com",
    "https://pub2.aplocoin.com",   # multiple URLs can be added for reliability
]

[wallet]
# Specify either seed_phrase or private_key + wallet_address
seed_phrase = "your 12 or 24-word seed phrase"
# private_key = "0xyour_private_key"
# wallet_address = "0xyour_address"

[contract]
contract_address = "0xHaplo_contract_address"   # e.g., 0x...

[miner]
max_wallets = 20                 # number of wallets to use
gas_thresholds = 0.01            # minimum balance to start (in GAPLO)
token_withdrawal_multiplier = 0.001   # fraction of tokens to withdraw to the main wallet
log_level = 2                    # 0 - minimum, 3 - maximum detail
mining_threads_per_wallet = 4    # threads per wallet
blocks_to_wait = 20              # number of blocks to wait between mining cycles
```
## 📄 Step 4. Preparing the contract ABI
An `abi.json` file—which describes the contract—must be located at the project root. If it is missing, obtain it from the repository or ask the community.

Ensure that the file is named exactly `abi.json` and is located in the same folder as `Cargo.toml`.

## 🛠️ Step 5. Assembling the miner
In the terminal (in the project folder), run:

```bash
cargo build --release
```
This will take a few minutes (longer the first time, as dependencies are being downloaded). Wait for the message: Finished release [optimized] target(s) in ....

## 🚀 Step 6. Launch
After building, start the miner:

```bash
cargo run --release
```
Alternatively, you can run the compiled file directly:

- Windows: target\release\lp-miner-gaplo.exe

- inux/macOS: ./target/release/lp-miner-gaplo

The miner will start creating wallets, funding them from the main wallet, and mining. Logs will be displayed in the console.
