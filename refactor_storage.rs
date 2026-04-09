use std::fs::{self, File};
use std::io::Write;

fn main() {
    let chunks = vec![
        ("sync_policy", "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum SyncPolicy {\n    Always,\n    Manual,\n}"),
        ("ttl_state", "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum TtlState {\n    NotFound,\n    NoExpire,\n    Seconds(i64),\n}"),
        ("repair_mode", "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum RepairMode {\n    Off,\n    Read,\n    Write,\n    Always,\n}\n\nimpl RepairMode {\n    pub fn from_input(raw: &str) -> Option<Self> {\n        match raw.to_ascii_lowercase().as_str() {\n            \"off\" => Some(Self::Off),\n            \"read\" => Some(Self::Read),\n            \"write\" => Some(Self::Write),\n            \"always\" => Some(Self::Always),\n            _ => None,\n        }\n    }\n\n    pub fn as_str(self) -> &'static str {\n        match self {\n            Self::Off => \"off\",\n            Self::Read => \"read\",\n            Self::Write => \"write\",\n            Self::Always => \"always\",\n        }\n    }\n}"),
        ("consistency_diff_kind", "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum ConsistencyDiffKind {\n    OnlyInCache,\n    OnlyInDisk,\n    ValueMismatch,\n}"),
        ("consistency_diff", "use crate::key_codec::KeyEncoding;\nuse super::consistency_diff_kind::ConsistencyDiffKind;\n\n#[derive(Debug, Clone, PartialEq, Eq)]\npub struct ConsistencyDiff {\n    pub key: KeyEncoding,\n    pub kind: ConsistencyDiffKind,\n}"),
        ("consistency_report", "use super::consistency_diff::ConsistencyDiff;\n\n#[derive(Debug, Clone, PartialEq, Eq)]\npub struct ConsistencyReport {\n    pub cache_keys: usize,\n    pub disk_keys: usize,\n    pub only_in_cache: usize,\n    pub only_in_disk: usize,\n    pub value_mismatches: usize,\n    pub samples: Vec<ConsistencyDiff>,\n}\n\nimpl ConsistencyReport {\n    pub fn total_issues(&self) -> usize {\n        self.only_in_cache + self.only_in_disk + self.value_mismatches\n    }\n}"),
        ("repair_target", "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum RepairTarget {\n    Disk,\n    Cache,\n}"),
        ("fault_target", "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum FaultTarget {\n    CacheOnly,\n    DiskOnly,\n}"),
        ("repair_report", "use super::repair_target::RepairTarget;\n\n#[derive(Debug, Clone, PartialEq, Eq)]\npub struct RepairReport {\n    pub target: RepairTarget,\n    pub repaired_only_in_cache: usize,\n    pub repaired_only_in_disk: usize,\n    pub repaired_value_mismatches: usize,\n}\n\nimpl RepairReport {\n    pub fn total_repairs(&self) -> usize {\n        self.repaired_only_in_cache + self.repaired_only_in_disk + self.repaired_value_mismatches\n    }\n}"),
        ("repair_summary", "use super::repair_target::RepairTarget;\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub struct RepairSummary {\n    pub target: RepairTarget,\n    pub repaired_only_in_cache: usize,\n    pub repaired_only_in_disk: usize,\n    pub repaired_value_mismatches: usize,\n}\n\nimpl RepairSummary {\n    pub fn total_repairs(&self) -> usize {\n        self.repaired_only_in_cache + self.repaired_only_in_disk + self.repaired_value_mismatches\n    }\n}"),
        ("engine_stats", "#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]\npub struct EngineStats {\n    pub reads: u64,\n    pub writes: u64,\n    pub deletes: u64,\n    pub cache_hits: u64,\n    pub cache_misses: u64,\n    pub disk_reads: u64,\n    pub disk_writes: u64,\n    pub wal_appends: u64,\n    pub fsync_count: u64,\n    pub ttl_expired_in_cache: u64,\n    pub ttl_expired_on_disk: u64,\n    pub ttl_loaded_on_startup: u64,\n    pub ttl_pruned_on_startup: u64,\n    pub cache_repaired: u64,\n    pub cache_invalidated: u64,\n    pub cache_evictions: u64,\n    pub auto_repairs: u64,\n    pub auto_repairs_read: u64,\n    pub auto_repairs_write: u64,\n}"),
        ("kv_engine", "use std::io;\nuse crate::key_codec::KeyEncoding;\n\npub trait KvEngine {\n    fn engine_name(&self) -> &'static str;\n    fn len(&mut self) -> usize;\n    fn get(&mut self, key: &KeyEncoding) -> Option<&[u8]>;\n    fn set(&mut self, key: KeyEncoding, value: Vec<u8>) -> io::Result<()>;\n    fn delete(&mut self, key: &KeyEncoding) -> io::Result<Option<Vec<u8>>>;\n}"),
        ("disk_read_engine", "use crate::key_codec::KeyEncoding;\nuse super::kv_engine::KvEngine;\n\npub trait DiskReadEngine: KvEngine {\n    fn get_disk_only(&mut self, key: &KeyEncoding) -> Option<Vec<u8>> {\n        self.get(key).map(|v| v.to_vec())\n    }\n}"),
        ("consistency_engine", "use std::io;\nuse super::consistency_report::ConsistencyReport;\n\npub trait ConsistencyEngine {\n    fn verify_consistency(&mut self) -> io::Result<ConsistencyReport> {\n        Err(io::Error::new(\n            io::ErrorKind::Unsupported,\n            \"consistency verify is not supported by this engine\",\n        ))\n    }\n}"),
        ("consistency_repair_engine", "use std::io;\nuse super::repair_target::RepairTarget;\nuse super::repair_report::RepairReport;\nuse super::repair_summary::RepairSummary;\n\npub trait ConsistencyRepairEngine {\n    fn repair_consistency(&mut self, _target: RepairTarget) -> io::Result<RepairReport> {\n        Err(io::Error::new(\n            io::ErrorKind::Unsupported,\n            \"consistency repair is not supported by this engine\",\n        ))\n    }\n\n    fn last_repair_summary(&self) -> Option<RepairSummary> {\n        None\n    }\n}"),
        ("repair_control_engine", "use std::io;\nuse super::repair_mode::RepairMode;\n\npub trait RepairControlEngine {\n    fn set_repair_mode(&mut self, _mode: RepairMode) -> io::Result<()> {\n        Err(io::Error::new(\n            io::ErrorKind::Unsupported,\n            \"repair mode is not supported by this engine\",\n        ))\n    }\n\n    fn repair_mode(&self) -> Option<RepairMode> {\n        None\n    }\n}"),
        ("fault_injection_engine", "use std::io;\nuse crate::key_codec::KeyEncoding;\nuse super::fault_target::FaultTarget;\n\npub trait FaultInjectionEngine {\n    fn inject_fault(\n        &mut self,\n        _target: FaultTarget,\n        _key: KeyEncoding,\n        _value: Vec<u8>,\n    ) -> io::Result<()> {\n        Err(io::Error::new(\n            io::ErrorKind::Unsupported,\n            \"fault injection is not supported by this engine\",\n        ))\n    }\n}"),
        ("ttl_engine", "use std::io;\nuse crate::key_codec::KeyEncoding;\nuse super::ttl_state::TtlState;\n\npub trait TtlEngine {\n    fn expire(&mut self, _key: &KeyEncoding, _seconds: u64) -> io::Result<bool> {\n        Ok(false)\n    }\n\n    fn ttl(&mut self, _key: &KeyEncoding) -> io::Result<TtlState> {\n        Ok(TtlState::NotFound)\n    }\n}"),
        ("durability_engine", "use std::io;\n\npub trait DurabilityEngine {\n    fn sync(&mut self) -> io::Result<()> {\n        Err(io::Error::new(\n            io::ErrorKind::Unsupported,\n            \"durability sync is not supported by this engine\",\n        ))\n    }\n\n    fn checkpoint(&mut self) -> io::Result<()> {\n        Err(io::Error::new(\n            io::ErrorKind::Unsupported,\n            \"durability checkpoint is not supported by this engine\",\n        ))\n    }\n}"),
        ("range_read_engine", "use std::io;\nuse crate::key_codec::KeyEncoding;\n\npub trait RangeReadEngine {\n    fn range(\n        &mut self,\n        _start: &KeyEncoding,\n        _end: &KeyEncoding,\n    ) -> io::Result<Vec<(KeyEncoding, Vec<u8>)>> {\n        Err(io::Error::new(\n            io::ErrorKind::Unsupported,\n            \"range read is not supported by this engine\",\n        ))\n    }\n}"),
        ("cache_capacity_engine", "use std::io;\n\npub trait CacheCapacityEngine {\n    fn set_cache_max_keys(&mut self, _max_keys: usize) -> io::Result<()> {\n        Err(io::Error::new(\n            io::ErrorKind::Unsupported,\n            \"cache max keys is not supported by this engine\",\n        ))\n    }\n\n    fn cache_max_keys(&self) -> Option<usize> {\n        None\n    }\n\n    fn cache_current_keys(&self) -> Option<usize> {\n        None\n    }\n}"),
        ("cache_policy_engine", "use std::io;\n\npub trait CachePolicyEngine {\n    fn set_cache_policy(&mut self, _policy: &str) -> io::Result<()> {\n        Err(io::Error::new(\n            io::ErrorKind::Unsupported,\n            \"cache policy is not supported by this engine\",\n        ))\n    }\n\n    fn cache_policy(&self) -> Option<&'static str> {\n        None\n    }\n}"),
        ("cache_config_engine", "use super::cache_capacity_engine::CacheCapacityEngine;\nuse super::cache_policy_engine::CachePolicyEngine;\n\npub trait CacheConfigEngine: CacheCapacityEngine + CachePolicyEngine {}\nimpl<T> CacheConfigEngine for T where T: CacheCapacityEngine + CachePolicyEngine {}"),
        ("storage_path_introspection", "use std::path::Path;\n\npub trait StoragePathIntrospection {\n    fn wal_path(&self) -> Option<&Path> {\n        None\n    }\n\n    fn snapshot_path(&self) -> Option<&Path> {\n        None\n    }\n}"),
        ("storage_stats_introspection", "use super::engine_stats::EngineStats;\n\npub trait StorageStatsIntrospection {\n    fn stats(&self) -> EngineStats {\n        EngineStats::default()\n    }\n}"),
        ("storage_introspection", "use super::storage_path_introspection::StoragePathIntrospection;\nuse super::storage_stats_introspection::StorageStatsIntrospection;\n\npub trait StorageIntrospection: StoragePathIntrospection + StorageStatsIntrospection {}\nimpl<T> StorageIntrospection for T where T: StoragePathIntrospection + StorageStatsIntrospection {}"),
        ("storage_engine", "use super::kv_engine::KvEngine;\nuse super::disk_read_engine::DiskReadEngine;\nuse super::range_read_engine::RangeReadEngine;\nuse super::consistency_engine::ConsistencyEngine;\nuse super::consistency_repair_engine::ConsistencyRepairEngine;\nuse super::repair_control_engine::RepairControlEngine;\nuse super::fault_injection_engine::FaultInjectionEngine;\nuse super::ttl_engine::TtlEngine;\nuse super::durability_engine::DurabilityEngine;\nuse super::cache_config_engine::CacheConfigEngine;\nuse super::storage_introspection::StorageIntrospection;\n\npub trait StorageEngine:\n    KvEngine\n    + DiskReadEngine\n    + RangeReadEngine\n    + ConsistencyEngine\n    + ConsistencyRepairEngine\n    + RepairControlEngine\n    + FaultInjectionEngine\n    + TtlEngine\n    + DurabilityEngine\n    + CacheConfigEngine\n    + StorageIntrospection\n{\n}\n\nimpl<T> StorageEngine for T where\n    T: KvEngine\n        + DiskReadEngine\n        + RangeReadEngine\n        + ConsistencyEngine\n        + ConsistencyRepairEngine\n        + RepairControlEngine\n        + FaultInjectionEngine\n        + TtlEngine\n        + DurabilityEngine\n        + CacheConfigEngine\n        + StorageIntrospection\n{\n}"),
    ];

    let mut mod_lines = vec![
        "pub mod hybrid_store;".to_string(),
        "pub mod memory_store;".to_string(),
        "pub mod wal_store;".to_string(),
        "pub mod disk_manager;".to_string(),
        "pub mod buffer_pool;".to_string(),
        "pub use hybrid_store::HybridStore;".to_string(),
        "pub use memory_store::MemoryStore;".to_string(),
        "pub use wal_store::WalStore;".to_string(),
    ];

    for (k, v) in chunks {
        let fpath = format!("src/storage/{}.rs", k);
        let mut f = File::create(&fpath).unwrap();
        f.write_all(v.as_bytes()).unwrap();
        
        mod_lines.push(format!("pub mod {};", k));
        mod_lines.push(format!("pub use {}::*;", k));
    }

    mod_lines.push("\n#[cfg(test)]\nmod consistency_tests;\n".to_string());

    let mut f = File::create("src/storage/mod.rs").unwrap();
    f.write_all(mod_lines.join("\n").as_bytes()).unwrap();
}
