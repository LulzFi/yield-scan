use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// The rpc http bind address
    #[arg(long, env, default_value = "127.0.0.1")]
    pub http_bind: String,

    /// The rpc http port
    #[arg(long, env, default_value = "8711")]
    pub http_port: u16,

    /// The open files limit
    #[arg(long, env, default_value = "10240")]
    pub open_files_limit: u64,

    /// The blockchain endpoint
    #[arg(long, env)]
    pub rpc_endpoint: String,

    /// The database file path
    #[arg(long, env, default_value = "db.sqlite")]
    pub db_path: String,
}

pub fn parse() -> Args {
    dotenv::dotenv().ok();
    let args = Args::parse();
    args
}
