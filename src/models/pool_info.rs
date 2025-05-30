use crate::{libs::config::JSON_CONFIG, yield_scaner::NATIVE_TOKEN_PRICE};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PoolInfoModel {
    pub protocol: String,
    pub factory: String,
    pub pool: String,
    pub token0: String,
    pub token1: String,
    pub fee: u64,
    pub token0_liquidity: u64,
    pub token1_liquidity: u64,
    pub timestamp: u64,
}

impl PoolInfoModel {
    pub fn get_liquidity(&self) -> f64 {
        let (token, liquidity) = if self.token0 == JSON_CONFIG.wrap_token || JSON_CONFIG.stable_tokens.contains_key(&self.token0) {
            (&self.token0, self.token0_liquidity as f64)
        } else if self.token1 == JSON_CONFIG.wrap_token || JSON_CONFIG.stable_tokens.contains_key(&self.token1) {
            (&self.token1, self.token1_liquidity as f64)
        } else {
            return 0.0;
        };

        let liquidity = if *token == JSON_CONFIG.wrap_token {
            liquidity * *NATIVE_TOKEN_PRICE.read().unwrap()
        } else {
            liquidity
        };

        liquidity / 10f64.powi(18 as i32)
    }
}
