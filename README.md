# Gaplo Miner (Rust)

A miner for Gaplo tokens on the APLO L1 network. Implemented in Rust using ethers-rs.

## Features

- RPC pool for fault tolerance
- Parallel nonce search (via `rayon`)
- Multi-wallet management
- Automatic balance top-up
- Metrics

## Installation and Execution

1. Install Rust: https://rustup.rs/
2. Clone the repository:
   ```bash
   git clone...
   cd gaplo-miner-rust
   ```
3. Configure config.toml (copy it from config.toml.example).

4. Place abi.json in the root directory.

5. Build and run:
```bash
cargo build --release
./target/release/lp-miner-gaplo
```
## Configuration
All parameters are in config.toml. Read more...
## License

Licensed under the [MIT License](LICENSE).
