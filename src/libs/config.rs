use super::{Tools, args::Args};
use crate::blockchain::ethereum::{Web3Client, init_web3_http};
use clap::Parser;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

lazy_static! {
    pub static ref ARGS: Args = Args::parse();
    pub static ref HTTP_BIND: String = ARGS.http_bind.clone();
    pub static ref HTTP_PORT: u16 = ARGS.http_port;
    pub static ref OPEN_FILES_LIMIT: u64 = ARGS.open_files_limit;
    pub static ref RPC_ENDPOINT: String = ARGS.rpc_endpoint.clone();
    pub static ref DB_PATH: String = ARGS.db_path.clone();
}

pub fn get_web3_rpc_client() -> Web3Client {
    init_web3_http(RPC_ENDPOINT.as_str())
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonConfig {
    pub wrap_token_pool: String,
    pub wrap_token: String,
    pub stable_tokens: HashMap<String, String>,
    pub swap_topics: HashMap<String, String>,
    pub factories: HashMap<String, String>,
}

pub static JSON_CONFIG: Lazy<JsonConfig> = Lazy::new(|| {
    let json_str = Tools::read_file_text("config.jsonc").unwrap();
    let config = jsonc_parser::parse_to_serde_value(&json_str, &Default::default()).unwrap().unwrap();
    serde_json::from_value(config).unwrap()
});
