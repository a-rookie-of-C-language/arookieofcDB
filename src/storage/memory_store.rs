use std::io;

use crate::engine::ArookieofcHashTable;

use super::{StorageEngine, SyncPolicy};

#[derive(Debug)]
pub struct MemoryStore {
    table: ArookieofcHashTable,
    sync_policy: SyncPolicy,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            table: ArookieofcHashTable::new(),
            sync_policy: SyncPolicy::Manual,
        }
    }
}

impl StorageEngine for MemoryStore {
    fn engine_name(&self) -> &'static str {
        "memory"
    }

    fn len(&self) -> usize {
        self.table.len()
    }

    fn get(&self, key: i64) -> Option<&[u8]> {
        self.table.get(key)
    }

    fn set(&mut self, key: i64, value: Vec<u8>) -> io::Result<()> {
        self.table.insert(key, value);
        Ok(())
    }

    fn delete(&mut self, key: i64) -> io::Result<Option<Vec<u8>>> {
        Ok(self.table.remove(key))
    }

    fn range_query(&self, start: i64, end: i64) -> Vec<(i64, Vec<u8>)> {
        self.table.range_query(start, end)
    }

    fn sync_policy(&self) -> SyncPolicy {
        self.sync_policy
    }

    fn set_sync_policy(&mut self, policy: SyncPolicy) {
        self.sync_policy = policy;
    }
}

