mod hybrid_store;
mod memory_store;
mod wal_store;
pub mod disk_manager;
pub mod buffer_pool;

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
pub enum RepairMode {
    Off,
    Read,
    Write,
    Always,
}

impl RepairMode {
    pub fn from_input(raw: &str) -> Option<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "off" => Some(Self::Off),
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "always" => Some(Self::Always),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Read => "read",
            Self::Write => "write",
            Self::Always => "always",
        }
    }
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
    pub ttl_loaded_on_startup: u64,
    pub ttl_pruned_on_startup: u64,
    pub cache_repaired: u64,
    pub cache_invalidated: u64,
    pub cache_evictions: u64,
    pub auto_repairs: u64,
    pub auto_repairs_read: u64,
    pub auto_repairs_write: u64,
}

pub trait KvEngine {
    fn engine_name(&self) -> &'static str;
    fn len(&mut self) -> usize;
    fn get(&mut self, key: &KeyEncoding) -> Option<&[u8]>;
    fn set(&mut self, key: KeyEncoding, value: Vec<u8>) -> io::Result<()>;
    fn delete(&mut self, key: &KeyEncoding) -> io::Result<Option<Vec<u8>>>;
}

pub trait DiskReadEngine: KvEngine {
    fn get_disk_only(&mut self, key: &KeyEncoding) -> Option<Vec<u8>> {
        self.get(key).map(|v| v.to_vec())
    }
}

pub trait ConsistencyEngine {
    fn verify_consistency(&mut self) -> io::Result<ConsistencyReport> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "consistency verify is not supported by this engine",
        ))
    }
}

pub trait ConsistencyRepairEngine {
    fn repair_consistency(&mut self, _target: RepairTarget) -> io::Result<RepairReport> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "consistency repair is not supported by this engine",
        ))
    }

    fn last_repair_summary(&self) -> Option<RepairSummary> {
        None
    }
}

pub trait RepairControlEngine {
    fn set_repair_mode(&mut self, _mode: RepairMode) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "repair mode is not supported by this engine",
        ))
    }

    fn repair_mode(&self) -> Option<RepairMode> {
        None
    }
}

pub trait FaultInjectionEngine {
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
}

pub trait TtlEngine {
    fn expire(&mut self, _key: &KeyEncoding, _seconds: u64) -> io::Result<bool> {
        Ok(false)
    }

    fn ttl(&mut self, _key: &KeyEncoding) -> io::Result<TtlState> {
        Ok(TtlState::NotFound)
    }
}

pub trait DurabilityEngine {
    fn sync(&mut self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "durability sync is not supported by this engine",
        ))
    }

    fn checkpoint(&mut self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "durability checkpoint is not supported by this engine",
        ))
    }
}

pub trait RangeReadEngine {
    fn range(
        &mut self,
        _start: &KeyEncoding,
        _end: &KeyEncoding,
    ) -> io::Result<Vec<(KeyEncoding, Vec<u8>)>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "range read is not supported by this engine",
        ))
    }
}

pub trait CacheCapacityEngine {
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
}

pub trait CachePolicyEngine {
    fn set_cache_policy(&mut self, _policy: &str) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "cache policy is not supported by this engine",
        ))
    }

    fn cache_policy(&self) -> Option<&'static str> {
        None
    }
}

pub trait CacheConfigEngine: CacheCapacityEngine + CachePolicyEngine {}

impl<T> CacheConfigEngine for T where T: CacheCapacityEngine + CachePolicyEngine {}

pub trait StoragePathIntrospection {
    fn wal_path(&self) -> Option<&Path> {
        None
    }

    fn snapshot_path(&self) -> Option<&Path> {
        None
    }
}

pub trait StorageStatsIntrospection {
    fn stats(&self) -> EngineStats {
        EngineStats::default()
    }
}

pub trait StorageIntrospection: StoragePathIntrospection + StorageStatsIntrospection {}

impl<T> StorageIntrospection for T where T: StoragePathIntrospection + StorageStatsIntrospection {}

pub trait StorageEngine:
    KvEngine
    + DiskReadEngine
    + RangeReadEngine
    + ConsistencyEngine
    + ConsistencyRepairEngine
    + RepairControlEngine
    + FaultInjectionEngine
    + TtlEngine
    + DurabilityEngine
    + CacheConfigEngine
    + StorageIntrospection
{
}

impl<T> StorageEngine for T where
    T: KvEngine
        + DiskReadEngine
        + RangeReadEngine
        + ConsistencyEngine
        + ConsistencyRepairEngine
        + RepairControlEngine
        + FaultInjectionEngine
        + TtlEngine
        + DurabilityEngine
        + CacheConfigEngine
        + StorageIntrospection
{
}

#[cfg(test)]
mod consistency_tests;
