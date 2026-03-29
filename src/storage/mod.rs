mod hybrid_store;
mod memory_store;
mod wal_store;

use std::io;
use std::path::Path;

use crate::key_codec::KeyEncoding;

pub use hybrid_store::HybridStore;
pub use memory_store::MemoryStore;
pub use wal_store::WalStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPolicy {
    Always,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtlState {
    NotFound,
    NoExpire,
    Seconds(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyDiffKind {
    OnlyInCache,
    OnlyInDisk,
    ValueMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyDiff {
    pub key: KeyEncoding,
    pub kind: ConsistencyDiffKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyReport {
    pub cache_keys: usize,
    pub disk_keys: usize,
    pub only_in_cache: usize,
    pub only_in_disk: usize,
    pub value_mismatches: usize,
    pub samples: Vec<ConsistencyDiff>,
}

impl ConsistencyReport {
    pub fn total_issues(&self) -> usize {
        self.only_in_cache + self.only_in_disk + self.value_mismatches
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairTarget {
    Disk,
    Cache,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultTarget {
    CacheOnly,
    DiskOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairReport {
    pub target: RepairTarget,
    pub repaired_only_in_cache: usize,
    pub repaired_only_in_disk: usize,
    pub repaired_value_mismatches: usize,
}

impl RepairReport {
    pub fn total_repairs(&self) -> usize {
        self.repaired_only_in_cache + self.repaired_only_in_disk + self.repaired_value_mismatches
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairSummary {
    pub target: RepairTarget,
    pub repaired_only_in_cache: usize,
    pub repaired_only_in_disk: usize,
    pub repaired_value_mismatches: usize,
}

impl RepairSummary {
    pub fn total_repairs(&self) -> usize {
        self.repaired_only_in_cache + self.repaired_only_in_disk + self.repaired_value_mismatches
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EngineStats {
    pub reads: u64,
    pub writes: u64,
    pub deletes: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub disk_reads: u64,
    pub disk_writes: u64,
    pub wal_appends: u64,
    pub fsync_count: u64,
    pub ttl_expired_in_cache: u64,
    pub ttl_expired_on_disk: u64,
    pub cache_repaired: u64,
    pub cache_invalidated: u64,
    pub cache_evictions: u64,
}

pub trait StorageEngine {
    fn engine_name(&self) -> &'static str;
    fn len(&mut self) -> usize;
    fn get(&mut self, key: &KeyEncoding) -> Option<&[u8]>;

    fn get_disk_only(&mut self, key: &KeyEncoding) -> Option<Vec<u8>> {
        self.get(key).map(|v| v.to_vec())
    }

    fn set(&mut self, key: KeyEncoding, value: Vec<u8>) -> io::Result<()>;
    fn delete(&mut self, key: &KeyEncoding) -> io::Result<Option<Vec<u8>>>;
    fn range_query(&self, start: &KeyEncoding, end: &KeyEncoding) -> Vec<(KeyEncoding, Vec<u8>)>;

    fn verify_consistency(&mut self) -> io::Result<ConsistencyReport> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "consistency verify is not supported by this engine",
        ))
    }

    fn repair_consistency(&mut self, _target: RepairTarget) -> io::Result<RepairReport> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "consistency repair is not supported by this engine",
        ))
    }

    fn inject_fault(
        &mut self,
        _target: FaultTarget,
        _key: KeyEncoding,
        _value: Vec<u8>,
    ) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "fault injection is not supported by this engine",
        ))
    }

    fn last_repair_summary(&self) -> Option<RepairSummary> {
        None
    }

    fn expire(&mut self, _key: &KeyEncoding, _seconds: u64) -> io::Result<bool> {
        Ok(false)
    }

    fn ttl(&mut self, _key: &KeyEncoding) -> io::Result<TtlState> {
        Ok(TtlState::NotFound)
    }

    fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn checkpoint(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn sync_policy(&self) -> SyncPolicy {
        SyncPolicy::Manual
    }

    fn set_sync_policy(&mut self, _policy: SyncPolicy) {}

    fn set_cache_max_keys(&mut self, _max_keys: usize) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "cache max keys is not supported by this engine",
        ))
    }

    fn cache_max_keys(&self) -> Option<usize> {
        None
    }

    fn cache_current_keys(&self) -> Option<usize> {
        None
    }

    fn set_cache_policy(&mut self, _policy: &str) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "cache policy is not supported by this engine",
        ))
    }

    fn cache_policy(&self) -> Option<&'static str> {
        None
    }

    fn wal_path(&self) -> Option<&Path> {
        None
    }

    fn snapshot_path(&self) -> Option<&Path> {
        None
    }

    fn stats(&self) -> EngineStats {
        EngineStats::default()
    }
}

#[cfg(test)]
mod consistency_tests;
