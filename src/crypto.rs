use ethers::types::{Address, U256};
use ethers::utils::keccak256;
use ethers::abi::encode_packed;

pub fn hash_nonce(
    nonce: U256,
    sender: Address,
    difficulty: U256,
    prev_hash: U256,
    total_mined: U256,
) -> U256 {
    let mut nonce_bytes = [0u8; 32];
    nonce.to_big_endian(&mut nonce_bytes);
    let packed = encode_packed(&[
        ethers::abi::Token::Address(sender),
        ethers::abi::Token::Bytes(nonce_bytes.to_vec()),
        ethers::abi::Token::Uint(difficulty),
        ethers::abi::Token::Uint(prev_hash),
        ethers::abi::Token::Uint(total_mined),
    ]).expect("packing should succeed");
    let hash = keccak256(packed);
    U256::from_big_endian(&hash)
}

pub fn generate_nonce() -> U256 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    U256::from_big_endian(&bytes)
}