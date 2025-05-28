use std::{collections::HashMap, sync::RwLock};

use crate::{
    blockchain::ethereum::{HexParseTrait, Web3Ex, web3_u256_to_i128},
    libs::{config::get_web3_rpc_client, db_sqlite::get_sqlite_pool},
    models::pool_info::PoolInfoModel,
};
use once_cell::sync::Lazy;
use web3::types::{Address, BlockId, Log, U256};
pub const UNISWAPV3_POOL_ABI: &str = include_str!("./blockchain/ethereum/abi/uniswapv3_pair.json");

const UNISWAPV3_SWAP_TOPIC: &str = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";
const PANCAKESWAPV3_SWAP_TOPIC: &str = "0x19b47279256b2a23a1665c810c8d55a1758940ee09377d4f8d26497a3577dc83";
const TOKEN_NATIVE: &str = "0xbb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c";
const TOKEN_USDT: &str = "0x55d398326f99059ff775485246999027b3197955";
const TOKEN_USDC: &str = "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d";

static POOLS: Lazy<RwLock<HashMap<String, PoolInfoModel>>> = Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PoolProtocol {
    UniswapV3,
    PancakeSwapV3,
}

impl PoolProtocol {
    pub fn to_string(&self) -> String {
        match self {
            PoolProtocol::UniswapV3 => "UniswapV3".to_string(),
            PoolProtocol::PancakeSwapV3 => "PancakeSwapV3".to_string(),
        }
    }
}

pub struct V3ScanWorker {}

impl V3ScanWorker {
    pub fn new() -> Self {
        V3ScanWorker {}
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn db_load() -> anyhow::Result<()> {
        let pools = sqlx::query_as::<_, PoolInfoModel>("SELECT * FROM pools")
            .fetch_all(get_sqlite_pool().as_ref())
            .await?;

        let mut pools_map = POOLS.write().unwrap();
        for pool in pools {
            pools_map.insert(pool.pool.clone(), pool);
        }
        Ok(())
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        tokio::spawn(async move {
            Self::loop_scan().await;
        });
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
        let block_receipts = web3.get_block_receiepts(blocknumber).await?;

        for receipt in block_receipts {
            log::info!("tx: {}", receipt.transaction_hash.to_hex_string());
            for log in receipt.logs {
                Self::parse_tx_log_v3_swap(&log).await?;
            }
        }

        Ok(())
    }

    pub async fn get_pool_info(pool_protocol: PoolProtocol, pool: Address) -> anyhow::Result<PoolInfoModel> {
        if let Some(pool_info) = POOLS.read().unwrap().get(&pool.to_hex_string()) {
            return Ok(pool_info.clone());
        }

        let web3 = get_web3_rpc_client();
        let fee_rate = web3.query_smart_comtract::<u64, _>(pool, UNISWAPV3_POOL_ABI, "fee", (), None).await?;
        let token0: Address = web3.query_smart_comtract::<Address, _>(pool, UNISWAPV3_POOL_ABI, "token0", (), None).await?;
        let token1: Address = web3.query_smart_comtract::<Address, _>(pool, UNISWAPV3_POOL_ABI, "token1", (), None).await?;

        let pool_info = PoolInfoModel {
            protocol: pool_protocol.to_string(),
            pool: pool.to_hex_string(),
            token0: token0.to_hex_string(),
            token1: token1.to_hex_string(),
            fee: fee_rate,
        };

        POOLS.write().unwrap().insert(pool.to_hex_string(), pool_info.clone());
        sqlx::query("INSERT INTO pools (protocol, pool, token0, token1, fee) VALUES (?, ?, ?, ?, ?)")
            .bind(&pool_info.protocol)
            .bind(&pool_info.pool)
            .bind(&pool_info.token0)
            .bind(&pool_info.token1)
            .bind(pool_info.fee as i32)
            .execute(get_sqlite_pool().as_ref())
            .await?;

        Ok(pool_info)
    }

    pub fn parse_tx_log_v3_swap_amount(log: &web3::types::Log) -> (i128, i128) {
        let data = log.data.0.as_slice();
        let amount0 = U256::from_big_endian(&data[32 * 0..32 * 1]);
        let amount1 = U256::from_big_endian(&data[32 * 1..32 * 2]);

        (web3_u256_to_i128(amount0), web3_u256_to_i128(amount1))
    }

    pub async fn parse_tx_log_v3_swap(tx_log: &Log) -> anyhow::Result<()> {
        if tx_log.topics.len() == 0 {
            return Ok(());
        }

        let topic = tx_log.topics[0].to_hex_string();
        let pool_protocol = if topic == UNISWAPV3_SWAP_TOPIC {
            PoolProtocol::UniswapV3
        } else if topic == PANCAKESWAPV3_SWAP_TOPIC {
            PoolProtocol::PancakeSwapV3
        } else {
            return Ok(());
        };

        let pool_info = Self::get_pool_info(pool_protocol, tx_log.address).await?;
        log::info!("pool: {} {}", pool_protocol.to_string(), tx_log.address.to_hex_string());
        let (amount0, amount1) = Self::parse_tx_log_v3_swap_amount(tx_log);
        let (token, amount) = if pool_info.token0 == TOKEN_NATIVE || pool_info.token0 == TOKEN_USDT || pool_info.token0 == TOKEN_USDC {
            (pool_info.token0, amount0.abs())
        } else if pool_info.token1 == TOKEN_NATIVE || pool_info.token1 == TOKEN_USDT || pool_info.token1 == TOKEN_USDC {
            (pool_info.token1, amount1.abs())
        } else {
            return Ok(());
        };

        let amount = if token == TOKEN_NATIVE {
            amount * 685 / 10i128.pow(18)
        } else {
            amount / 10i128.pow(18)
        };
        log::info!(
            "Pool: {}, Token: {}, Amount: {}, Protocol: {}",
            pool_info.pool,
            token,
            amount,
            pool_protocol.to_string()
        );

        Ok(())
    }
}
