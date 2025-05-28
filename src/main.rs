use dotenv::dotenv;
use log::info;
use tokio;
use v3scan::{
    api,
    libs::{adjust_open_files, config, log::init_log},
    v3_scan::V3ScanWorker,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    init_log();

    info!("{:?}", *config::ARGS);

    adjust_open_files::adjust_open_files_limit();

    ctrlc::set_handler(|| {
        info!("Received Ctrl+C signal. Exiting...");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    api::server::run(false).await;

    let app = V3ScanWorker::new();
    app.init().await.unwrap();

    app.run().await.unwrap();

    Ok(())
}
