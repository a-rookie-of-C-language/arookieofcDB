use crate::storage::EngineStats;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSnapshot {
    pub engine: &'static str,
    pub len: usize,
    pub stats: EngineStats,
    pub wal_path: String,
    pub wal_bytes: u64,
    pub snapshot_path: String,
    pub snapshot_bytes: u64,
    pub snapshot_mtime_unix: String,
    pub cache_policy: String,
    pub cache_max_keys: String,
    pub cache_current_keys: String,
    pub repair_mode: String,
    pub inconsistency_total: String,
    pub inconsistency_only_in_cache: String,
    pub inconsistency_only_in_disk: String,
    pub inconsistency_value_mismatch: String,
    pub last_repair_target: String,
    pub last_repair_total: String,
    pub last_repair_only_in_cache: String,
    pub last_repair_only_in_disk: String,
    pub last_repair_value_mismatch: String,
}
