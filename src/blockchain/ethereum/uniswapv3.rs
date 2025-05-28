use super::Web3Ex;
use crate::libs::config::get_web3_rpc_client;
use web3::types::{Address, BlockId, U256};

const UNISWAPV3_PAIR_ABI: &str = include_str!("./abi/uniswapv3_pair.json");

pub async fn calc_pool_price(pool: Address, block: Option<BlockId>) -> web3::contract::Result<f64> {
    let client = get_web3_rpc_client();
    let result: (U256, i32, u16, u16, u16, u8, bool) = client.query_smart_comtract(pool, UNISWAPV3_PAIR_ABI, "slot0", (), block).await?;
    let sqrt_price_x96 = result.0.as_u128() as f64;
    let q96 = 2.0f64.powi(96);
    let sqrt_price = sqrt_price_x96 / q96;
    let price = 1.0 / (sqrt_price * sqrt_price);
    Ok(price)
}
