use sqlx::{Pool, Sqlite, SqlitePool, sqlite::SqliteConnectOptions};
use std::sync::Arc;
use tokio::sync::OnceCell;

use super::{Tools, config::DB_PATH};

pub static DB_SQLITE: OnceCell<Arc<Pool<Sqlite>>> = OnceCell::const_new();

pub fn get_sqlite_pool() -> Arc<Pool<Sqlite>> {
    DB_SQLITE.get().unwrap().clone()
}

pub async fn sqlite_init() -> anyhow::Result<()> {
    let options = SqliteConnectOptions::new().filename(DB_PATH.as_str()).create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;
    DB_SQLITE.set(Arc::new(pool))?;

    let init = Tools::read_file_text("./init.sql")?;
    sqlx::query(&init).execute(get_sqlite_pool().as_ref()).await?;
    Ok(())
}
