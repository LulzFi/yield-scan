use crate::{
    blockchain::ethereum::{HexParseTrait, Web3Ex, uniswapv3, web3_u256_to_i128},
    libs::{
        Tools,
        config::{JSON_CONFIG, get_web3_rpc_client},
        db_sqlite::get_sqlite_pool,
        global::{LoopResult, get_timestamp, set_loop_global},
    },
    models::pool_info::PoolInfoModel,
};
use once_cell::sync::Lazy;
use std::{
    collections::{HashMap, VecDeque},
    sync::RwLock,
};
use web3::types::{Address, Block, BlockId, H256, Log, U256};

const UNISWAPV3_POOL_ABI: &str = include_str!("./blockchain/ethereum/abi/uniswapv3_pair.json");
const VOLUME_MINUTES_CACHE_SIZE: usize = 10;

static POOLS: Lazy<RwLock<HashMap<String, PoolInfoModel>>> = Lazy::new(|| RwLock::new(HashMap::new()));
pub static NATIVE_TOKEN_PRICE: RwLock<f64> = RwLock::new(0.0);
static VOLUME_CACHE: Lazy<RwLock<HashMap<String, VecDeque<(u64, u64)>>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub struct V3ScanWorker;

impl V3ScanWorker {
    pub fn new() -> Self {
        V3ScanWorker {}
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        Self::db_load().await?;
        Self::load_volume_cache()?;
        Self::loop_update_native_token_price().await?;
        Ok(())
    }

    pub fn load_volume_cache() -> anyhow::Result<()> {
        if let Ok(data) = Tools::read_file_text("volume_cache.json") {
            let volume_cache: HashMap<String, VecDeque<(u64, u64)>> = serde_json::from_str(&data)?;
            *VOLUME_CACHE.write().unwrap() = volume_cache;
            log::info!("Loaded {} pools volume cache from file", VOLUME_CACHE.read().unwrap().len());
        } else {
            log::info!("No volume cache file found");
        }
        Ok(())
    }

    pub async fn save_volume_cache() -> LoopResult {
        let volume_cache = VOLUME_CACHE.read().unwrap();
        let data = serde_json::to_string(&*volume_cache)?;
        Tools::write_file_text("volume_cache.json", &data)?;
        log::info!("Saved {} pools volume cache to file", volume_cache.len());
        Ok(())
    }

    pub async fn db_load() -> anyhow::Result<()> {
        let pools = sqlx::query_as::<_, PoolInfoModel>("SELECT * FROM pools")
            .fetch_all(get_sqlite_pool().as_ref())
            .await?;

        log::info!("Load {} pools from database", pools.len());
        let mut pools_map = POOLS.write().unwrap();
        for pool in pools {
            pools_map.insert(pool.pool.clone(), pool);
        }
        Ok(())
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        set_loop_global(Self::loop_update_native_token_price, 60 * 1000);
        set_loop_global(Self::save_volume_cache, 10 * 1000);
        set_loop_global(Self::loop_sort_yield, 60 * 1000);
        tokio::spawn(async move {
            Self::loop_scan().await;
        });
        Ok(())
    }

    pub async fn loop_update_native_token_price() -> LoopResult {
        let price = uniswapv3::calc_pool_price(JSON_CONFIG.wrap_token_pool.parse::<Address>().unwrap(), None).await?;
        *NATIVE_TOKEN_PRICE.write().unwrap() = price;
        log::info!("Update native token price: {}", price);
        Ok(())
    }

    pub async fn loop_scan() {
        let web3 = get_web3_rpc_client();
        let mut work_blocknumber = web3.get_blocknumber_wait().await;
        loop {
            let current_blocknumber = web3.get_blocknumber_wait().await;
            if work_blocknumber > current_blocknumber {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }

            log::info!("Scann block: {}", work_blocknumber);
            let block_id: BlockId = BlockId::Number(work_blocknumber.into());
            match Self::yield_scan(block_id).await {
                Ok(_) => {
                    work_blocknumber += 1;
                }
                Err(e) => {
                    log::error!("Error scanning block {}: {}", work_blocknumber, e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    pub async fn yield_scan(blocknumber: BlockId) -> anyhow::Result<()> {
        let web3 = get_web3_rpc_client();
        let block = web3.eth().block(blocknumber).await?;
        let block_receipts = web3.get_block_receiepts(blocknumber).await?;

        let block = block.ok_or_else(|| anyhow::anyhow!("Block not found"))?;
        for receipt in block_receipts {
            // log::info!("tx: {}", receipt.transaction_hash.to_hex_string());
            for log in receipt.logs {
                Self::parse_tx_log_v3_swap(&block, &log).await?;
            }
        }

        Ok(())
    }

    pub async fn get_pool_info(pool_protocol: &str, pool: Address) -> anyhow::Result<Option<PoolInfoModel>> {
        const LIQUIDITY_TIMEOUT: u64 = 5 * 60;
        let mut pool_info = if let Some(pool_info) = POOLS.read().unwrap().get(&pool.to_hex_string()) {
            if get_timestamp() - pool_info.timestamp < LIQUIDITY_TIMEOUT {
                return Ok(Some(pool_info.clone()));
            }
            pool_info.clone()
        } else {
            if let Some(pool_info) = Self::get_pool_info_web3(pool_protocol, pool).await? {
                pool_info
            } else {
                return Ok(None);
            }
        };

        if get_timestamp() - pool_info.timestamp > LIQUIDITY_TIMEOUT {
            let web31 = get_web3_rpc_client();
            let web32 = get_web3_rpc_client();
            let (token0_liquidity, token1_liquidity) = futures::try_join!(
                web31.get_erc20_balance(pool_info.token0.parse::<Address>()?, pool),
                web32.get_erc20_balance(pool_info.token1.parse::<Address>()?, pool)
            )?;

            pool_info.token0_liquidity = (token0_liquidity.as_u128() / (10i128.pow(18) as u128)) as u64;
            pool_info.token1_liquidity = (token1_liquidity.as_u128() / (10i128.pow(18) as u128)) as u64;
            pool_info.timestamp = get_timestamp();
        }

        POOLS.write().unwrap().insert(pool.to_hex_string(), pool_info.clone());
        sqlx::query(
            "INSERT OR REPLACE INTO pools (protocol, pool, factory, token0, token1, fee, token0_liquidity, token1_liquidity, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&pool_info.protocol)
        .bind(&pool_info.pool)
        .bind(&pool_info.factory)
        .bind(&pool_info.token0)
        .bind(&pool_info.token1)
        .bind(pool_info.fee as i32)
        .bind(pool_info.token0_liquidity as i64)
        .bind(pool_info.token1_liquidity as i64)
        .bind(pool_info.timestamp as i64)
        .execute(get_sqlite_pool().as_ref())
        .await?;

        Ok(Some(pool_info))
    }

    pub async fn get_pool_info_web3(pool_protocol: &str, pool: Address) -> anyhow::Result<Option<PoolInfoModel>> {
        let web31 = get_web3_rpc_client();
        let web32 = get_web3_rpc_client();
        let web33 = get_web3_rpc_client();

        log::info!("Get pool info: {} {}", pool_protocol, pool.to_hex_string());
        let factory = web31.query_smart_contract::<Address, _>(pool, UNISWAPV3_POOL_ABI, "factory", (), None).await?;
        if !JSON_CONFIG.factories.contains_key(&factory.to_hex_string()) {
            return Ok(None);
        }

        let (fee_rate, token0, token1) = futures::try_join!(
            web31.query_smart_contract::<u64, _>(pool, UNISWAPV3_POOL_ABI, "fee", (), None),
            web32.query_smart_contract::<Address, _>(pool, UNISWAPV3_POOL_ABI, "token0", (), None),
            web33.query_smart_contract::<Address, _>(pool, UNISWAPV3_POOL_ABI, "token1", (), None)
        )?;

        log::info!(
            "Pool: {}, Token0: {}, Token1: {}, Fee: {}",
            pool.to_hex_string(),
            token0.to_hex_string(),
            token1.to_hex_string(),
            fee_rate
        );

        let divisor = U256::exp10(18);
        let (token0_liquidity, token1_liquidity) = futures::try_join!(web31.get_erc20_balance(token0, pool), web32.get_erc20_balance(token1, pool))?;
        let token0_liquidity = token0_liquidity / divisor;
        let token1_liquidity = token1_liquidity / divisor;

        let pool_info = PoolInfoModel {
            protocol: pool_protocol.to_string(),
            factory: factory.to_hex_string(),
            pool: pool.to_hex_string(),
            token0: token0.to_hex_string(),
            token1: token1.to_hex_string(),
            fee: fee_rate,
            token0_liquidity: token0_liquidity.as_u64(),
            token1_liquidity: token1_liquidity.as_u64(),
            timestamp: get_timestamp(),
        };

        Ok(Some(pool_info))
    }

    pub fn parse_tx_log_v3_swap_amount(log: &web3::types::Log) -> (i128, i128) {
        let data = log.data.0.as_slice();
        let amount0 = U256::from_big_endian(&data[32 * 0..32 * 1]);
        let amount1 = U256::from_big_endian(&data[32 * 1..32 * 2]);

        (web3_u256_to_i128(amount0), web3_u256_to_i128(amount1))
    }

    pub async fn parse_tx_log_v3_swap(block: &Block<H256>, tx_log: &Log) -> anyhow::Result<()> {
        if tx_log.topics.len() == 0 {
            return Ok(());
        }

        let topic = tx_log.topics[0].to_hex_string();
        let pool_protocol = if let Some(protocol) = JSON_CONFIG.swap_topics.get(&topic) {
            protocol
        } else {
            return Ok(());
        };

        let Some(pool_info) = Self::get_pool_info(pool_protocol, tx_log.address).await? else {
            return Ok(());
        };

        let (amount0, amount1) = Self::parse_tx_log_v3_swap_amount(tx_log);
        let (token, liquidity, amount) = if pool_info.token0 == JSON_CONFIG.wrap_token || JSON_CONFIG.stable_tokens.contains_key(&pool_info.token0) {
            (pool_info.token0, pool_info.token0_liquidity as f64, amount0.abs() as f64)
        } else if pool_info.token1 == JSON_CONFIG.wrap_token || JSON_CONFIG.stable_tokens.contains_key(&pool_info.token1) {
            (pool_info.token1, pool_info.token1_liquidity as f64, amount1.abs() as f64)
        } else {
            return Ok(());
        };

        if liquidity < 1000.0 {
            return Ok(());
        }

        let (amount, liquidity) = if token == JSON_CONFIG.wrap_token {
            (amount * *NATIVE_TOKEN_PRICE.read().unwrap(), liquidity * *NATIVE_TOKEN_PRICE.read().unwrap())
        } else {
            (amount, liquidity)
        };

        let amount = amount / (10i128.pow(18) as f64);
        if !amount.is_normal() {
            return Ok(());
        }

        let ts_min = block.timestamp.as_u64() / 60;
        let mut volume_cache = VOLUME_CACHE.write().unwrap();
        let pool_volume = volume_cache.entry(pool_info.pool.clone()).or_default();
        if let Some((last_ts, last_amount)) = pool_volume.back_mut() {
            if *last_ts == ts_min {
                *last_amount += amount as u64;
            } else {
                pool_volume.push_back((ts_min, amount as u64));
            }
        } else {
            pool_volume.push_back((ts_min, amount as u64));
        }

        if pool_volume.len() > VOLUME_MINUTES_CACHE_SIZE {
            pool_volume.pop_front();
        }

        let total_volume: u64 = pool_volume.iter().map(|(_, amt)| *amt).sum();
        let total_fee_cache = pool_info.fee * total_volume / 1000000;
        let total_fee_hour = ((total_fee_cache as f64) / (VOLUME_MINUTES_CACHE_SIZE as f64)) * 60.0;
        let fee_rate_per_hour = total_fee_hour as f64 / liquidity as f64;

        log::info!(
            "-{}s Pool: {}, Fee: {} Amount: {}, APH: {} TotalVolume: {} Liquidity: {}",
            get_timestamp() - block.timestamp.as_u64(),
            pool_info.pool,
            pool_info.fee,
            amount,
            fee_rate_per_hour,
            total_volume,
            liquidity as u64
        );

        if fee_rate_per_hour > 0.1 {
            log::info!(
                "high yield pool: {} pool: {} token: {} APH: {} Liquidity: {}",
                pool_protocol,
                pool_info.pool,
                token,
                fee_rate_per_hour,
                liquidity as u64
            );
        }
        Ok(())
    }

    pub async fn loop_sort_yield() -> LoopResult {
        let all_pool_info = POOLS.read().unwrap();
        let all_pool_volume = VOLUME_CACHE.read().unwrap();
        let mut pools: Vec<(String, f64, f64, f64)> = all_pool_volume
            .iter()
            .filter_map(|(pool, volumes)| {
                let Some(pool_info) = all_pool_info.get(pool) else {
                    return None;
                };

                let liquidity = pool_info.get_liquidity();
                if liquidity < 10000.0 {
                    return None;
                }

                let total_volume: u64 = volumes.iter().map(|(_, amt)| *amt).sum();
                if total_volume < 10000 {
                    return None;
                }

                let total_fee_cache = pool_info.fee * total_volume / 1000000;
                let total_fee_hour = ((total_fee_cache as f64) / (VOLUME_MINUTES_CACHE_SIZE as f64)) * 60.0;
                let fee_rate_per_hour = total_fee_hour as f64 / liquidity as f64;

                Some((pool.clone(), total_volume as f64, liquidity, fee_rate_per_hour))
            })
            .collect();
        pools.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
        pools.truncate(10);
        log::info!("Top 10 pools by fee rate per hour:");
        for (pool, total_volume, liquidity, fee_rate_per_hour) in pools {
            log::info!(
                "Pool: {}, Volume: {:.2}, Liquidity: {:.2}, APH: {:.6}",
                pool,
                total_volume,
                liquidity,
                fee_rate_per_hour
            );
        }

        Ok(())
    }
}
