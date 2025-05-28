use super::args::Args;
use crate::blockchain::ethereum::{Web3Client, init_web3_http};
use clap::Parser;

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
