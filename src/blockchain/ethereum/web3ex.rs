use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use web3::{
    Transport, Web3,
    contract::{
        Contract, Options,
        tokens::{Detokenize, Tokenize},
    },
    helpers,
    types::{Address, Block, BlockId, FilterBuilder, H256, Log, TransactionReceipt, U256},
};

pub type Web3HttpClient = web3::Web3<web3::transports::Http>;
pub const ERC20_ABI: &str = include_str!("./abi/erc20.json");

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockWithReceipts {
    pub block: Block<H256>,
    pub receipts: Vec<TransactionReceipt>,
}

pub struct ERC20TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u64,
    pub total_supply: U256,
}

#[async_trait]
pub trait Web3Ex<T: Transport + Send + Sync> {
    async fn get_chain_id(&self) -> u64;
    async fn get_blocknumber_wait(&self) -> u64;
    async fn get_block_receiepts(&self, blocknumber: BlockId) -> web3::Result<Vec<TransactionReceipt>>;
    async fn get_event_logs(&self, contracts: &Vec<String>, blocknumber: u64) -> web3::Result<Vec<Log>>;
    async fn get_erc20_balance(&self, contract_address: Address, address: Address) -> web3::contract::Result<U256>;
    async fn get_erc20_info(&self, token_contract: Address) -> web3::contract::Result<ERC20TokenInfo>;
    async fn query_smart_comtract<R, P>(
        &self,
        contract_address: Address,
        abi: &str,
        method: &str,
        params: P,
        blocknumber: Option<BlockId>,
    ) -> web3::contract::Result<R>
    where
        R: Detokenize,
        P: Tokenize + Send;
}

#[async_trait]
impl<T: Transport + Send + Sync> Web3Ex<T> for Web3<T>
where
    T::Out: Send,
{
    async fn get_chain_id(&self) -> u64 {
        self.eth().chain_id().await.unwrap().as_u64()
    }

    async fn get_blocknumber_wait(&self) -> u64 {
        loop {
            if let Ok(blocknumber) = self.eth().block_number().await {
                return blocknumber.as_u64();
            } else {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }

    async fn get_block_receiepts(&self, blocknumber: BlockId) -> web3::Result<Vec<TransactionReceipt>> {
        let blocknumber = helpers::serialize(&blocknumber);
        let result = self.transport().execute("eth_getBlockReceipts", vec![blocknumber]).await?;
        let receipts: Vec<TransactionReceipt> = serde_json::from_value(result)?;
        Ok(receipts)
    }

    async fn get_event_logs(&self, contracts: &Vec<String>, blocknumber: u64) -> web3::Result<Vec<Log>> {
        let filter = FilterBuilder::default()
            .address(contracts.iter().map(|x| x.parse().unwrap()).collect())
            .from_block(blocknumber.into())
            .to_block(blocknumber.into())
            .build();

        self.eth().logs(filter).await
    }

    async fn get_erc20_balance(&self, contract_address: Address, address: Address) -> web3::contract::Result<U256> {
        self.query_smart_comtract::<U256, (Address,)>(contract_address, ERC20_ABI, "balanceOf", (address,), None)
            .await
    }

    async fn query_smart_comtract<R, P>(
        &self,
        contract_address: Address,
        abi: &str,
        method: &str,
        params: P,
        blocknumber: Option<BlockId>,
    ) -> web3::contract::Result<R>
    where
        R: Detokenize,
        P: Tokenize + Send,
    {
        let contract = Contract::from_json(self.eth(), contract_address, abi.as_bytes()).unwrap();
        let result: R = contract.query(method, params, None, Options::default(), blocknumber).await?;
        Ok(result)
    }

    async fn get_erc20_info(&self, token_contract: Address) -> web3::contract::Result<ERC20TokenInfo> {
        match futures::try_join!(
            self.query_smart_comtract::<String, _>(token_contract, ERC20_ABI, "name", (), None),
            self.query_smart_comtract::<String, _>(token_contract, ERC20_ABI, "symbol", (), None),
            self.query_smart_comtract::<u64, _>(token_contract, ERC20_ABI, "decimals", (), None),
            self.query_smart_comtract::<U256, _>(token_contract, ERC20_ABI, "totalSupply", (), None)
        ) {
            Ok((name, symbol, decimals, total_supply)) => Ok(ERC20TokenInfo {
                name: name.trim_end_matches('\0').to_string(),
                symbol: symbol.trim_end_matches('\0').to_string(),
                decimals,
                total_supply,
            }),
            Err(e) => Err(e),
        }
    }
}
