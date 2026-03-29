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
