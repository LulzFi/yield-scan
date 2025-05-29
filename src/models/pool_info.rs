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
