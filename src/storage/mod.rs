mod memory_store;
mod wal_store;

use std::io;
use std::path::Path;

pub use memory_store::MemoryStore;
pub use wal_store::WalStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPolicy {
    Always,
    Manual,
}

pub trait StorageEngine {
    fn engine_name(&self) -> &'static str;
    fn len(&self) -> usize;
    fn get(&self, key: i64) -> Option<&[u8]>;
    fn set(&mut self, key: i64, value: Vec<u8>) -> io::Result<()>;
    fn delete(&mut self, key: i64) -> io::Result<Option<Vec<u8>>>;
    fn range_query(&self, start: i64, end: i64) -> Vec<(i64, Vec<u8>)>;

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

    fn wal_path(&self) -> Option<&Path> {
        None
    }

    fn snapshot_path(&self) -> Option<&Path> {
        None
    }
}
