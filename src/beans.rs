use crate::storage::db_engine::DbEngine;
use crate::wal::wal_config::WalConfig;
use macros::bean;

#[bean(name = "DbEngine")]
pub fn create_db_engine() -> DbEngine {
    let config = WalConfig::new("./data/wal");
    DbEngine::new(config).expect("Failed to create DbEngine")
}
