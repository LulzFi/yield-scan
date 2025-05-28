pub mod event_log;
pub mod traits;
pub mod uniswapv3;
pub mod web3_reqwest;
pub mod web3ex;

pub use traits::*;
use web3_reqwest::ReqwestTransport;
pub use web3ex::*;

pub const ETH_DECIMALS: u64 = 18;

pub type Web3Client = web3::Web3<ReqwestTransport>;

pub fn init_web3_http(url: &str) -> Web3Client {
    assert!(url.starts_with("http"));
    let transport = ReqwestTransport::new(url);
    let web3 = web3::Web3::new(transport);
    web3
}

pub fn web3_u256_to_i128(value: web3::types::U256) -> i128 {
    let mut bytes = [0u8; 32];
    value.to_big_endian(&mut bytes);
    let ethers_u256 = web3::types::U256::from_big_endian(&bytes);
    let i256 = ethers_core::types::I256::from_raw(ethers_u256);
    i256.as_i128()
}
