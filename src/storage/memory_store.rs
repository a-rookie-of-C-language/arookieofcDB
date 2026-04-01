use std::collections::{HashMap, VecDeque};
use std::io;
use std::time::{Duration, Instant};

use crate::engine::ArookieofcHashTable;
use crate::key_codec::KeyEncoding;

use super::{
    CacheCapacityEngine, CachePolicyEngine, ConsistencyEngine, ConsistencyRepairEngine,
    DiskReadEngine, EngineStats, FaultInjectionEngine, KvEngine, RangeReadEngine,
    RepairControlEngine, StoragePathIntrospection, StorageStatsIntrospection, TtlEngine,
    TtlState,
};

#[derive(Debug)]
pub struct MemoryStore {
    table: ArookieofcHashTable,
    expires: HashMap<KeyEncoding, Instant>,
    lru_order: VecDeque<KeyEncoding>,
    cache_max_keys: usize,
    cache_evictions: u64,
    stats: EngineStats,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            table: ArookieofcHashTable::new(),
            expires: HashMap::new(),
            lru_order: VecDeque::new(),
            cache_max_keys: 0,
            cache_evictions: 0,
            stats: EngineStats::default(),
        }
    }

    pub fn set_cache_max_keys(&mut self, max_keys: usize) {
        self.cache_max_keys = max_keys;
        self.enforce_capacity();
    }

    pub fn cache_max_keys(&self) -> usize {
        self.cache_max_keys
    }

    pub fn cache_current_keys(&self) -> usize {
        self.table.len()
    }

    pub fn cache_evictions(&self) -> u64 {
        self.cache_evictions
    }

    pub fn clear_cache(&mut self) {
        self.table = ArookieofcHashTable::new();
        self.expires.clear();
        self.lru_order.clear();
    }

    pub fn get_with_expiry_flag(&mut self, key: &KeyEncoding) -> (Option<Vec<u8>>, bool) {
        let had_deadline = self.expires.contains_key(key);
        let existed_before = self.table.get(key).is_some();
        self.purge_if_expired(key);
        let value = self.table.get(key).map(|v| v.to_vec());
        if value.is_some() {
            self.touch_key(key);
        }
        let expired_now = had_deadline && existed_before && value.is_none();
        (value, expired_now)
    }

    pub fn entries(&mut self) -> Vec<(KeyEncoding, Vec<u8>)> {
        self.purge_all_expired();
        self.table.entries()
    }

    fn touch_key(&mut self, key: &KeyEncoding) {
        if let Some(pos) = self.lru_order.iter().position(|k| k == key) {
            let _ = self.lru_order.remove(pos);
        }
        self.lru_order.push_back(key.clone());
    }

    fn remove_from_lru(&mut self, key: &KeyEncoding) {
        if let Some(pos) = self.lru_order.iter().position(|k| k == key) {
            let _ = self.lru_order.remove(pos);
        }
    }

    fn enforce_capacity(&mut self) {
        if self.cache_max_keys == 0 {
            return;
        }

        while self.table.len() > self.cache_max_keys {
            let Some(oldest) = self.lru_order.pop_front() else {
                break;
            };

            if self.table.remove(&oldest).is_some() {
                self.expires.remove(&oldest);
                self.cache_evictions += 1;
            }
        }
    }

    fn purge_if_expired(&mut self, key: &KeyEncoding) {
        let expired = self.expires.get(key).is_some_and(|deadline| Instant::now() >= *deadline);
        if expired {
            self.expires.remove(key);
            let _ = self.table.remove(key);
            self.remove_from_lru(key);
        }
    }

    fn purge_all_expired(&mut self) {
        let now = Instant::now();
        let expired_keys = self
            .expires
            .iter()
            .filter_map(|(k, deadline)| if now >= *deadline { Some(k.clone()) } else { None })
            .collect::<Vec<_>>();

        for key in expired_keys {
            self.expires.remove(&key);
            let _ = self.table.remove(&key);
            self.remove_from_lru(&key);
        }
    }
}

impl KvEngine for MemoryStore {
    fn engine_name(&self) -> &'static str {
        "memory"
    }

    fn len(&mut self) -> usize {
        self.purge_all_expired();
        self.table.len()
    }

    fn get(&mut self, key: &KeyEncoding) -> Option<&[u8]> {
        self.stats.reads += 1;
        self.purge_if_expired(key);
        if self.table.get(key).is_some() {
            self.touch_key(key);
        }
        self.table.get(key)
    }

    fn set(&mut self, key: KeyEncoding, value: Vec<u8>) -> io::Result<()> {
        self.stats.writes += 1;
        self.table.insert(key.clone(), value);
        self.expires.remove(&key);
        self.touch_key(&key);
        self.enforce_capacity();
        Ok(())
    }

    fn delete(&mut self, key: &KeyEncoding) -> io::Result<Option<Vec<u8>>> {
        self.stats.deletes += 1;
        self.expires.remove(key);
        self.remove_from_lru(key);
        Ok(self.table.remove(key))
    }
}

impl DiskReadEngine for MemoryStore {}

impl RangeReadEngine for MemoryStore {
    fn range(
        &mut self,
        start: &KeyEncoding,
        end: &KeyEncoding,
    ) -> io::Result<Vec<(KeyEncoding, Vec<u8>)>> {
        self.purge_all_expired();
        Ok(self
            .entries()
            .into_iter()
            .filter(|(key, _)| key >= start && key <= end)
            .collect())
    }
}

impl ConsistencyEngine for MemoryStore {}

impl ConsistencyRepairEngine for MemoryStore {}

impl RepairControlEngine for MemoryStore {}

impl FaultInjectionEngine for MemoryStore {}

impl TtlEngine for MemoryStore {
    fn expire(&mut self, key: &KeyEncoding, seconds: u64) -> io::Result<bool> {
        self.purge_if_expired(key);
        if self.table.get(key).is_none() {
            return Ok(false);
        }

        let deadline = Instant::now() + Duration::from_secs(seconds);
        self.expires.insert(key.clone(), deadline);
        Ok(true)
    }

    fn ttl(&mut self, key: &KeyEncoding) -> io::Result<TtlState> {
        self.purge_if_expired(key);

        if self.table.get(key).is_none() {
            return Ok(TtlState::NotFound);
        }

        let Some(deadline) = self.expires.get(key) else {
            return Ok(TtlState::NoExpire);
        };

        let now = Instant::now();
        if *deadline <= now {
            return Ok(TtlState::NotFound);
        }

        let remain = deadline.duration_since(now).as_secs() as i64;
        Ok(TtlState::Seconds(remain))
    }
}

impl CacheCapacityEngine for MemoryStore {
    fn set_cache_max_keys(&mut self, max_keys: usize) -> io::Result<()> {
        MemoryStore::set_cache_max_keys(self, max_keys);
        Ok(())
    }

    fn cache_max_keys(&self) -> Option<usize> {
        Some(self.cache_max_keys())
    }

    fn cache_current_keys(&self) -> Option<usize> {
        Some(self.cache_current_keys())
    }
}

impl CachePolicyEngine for MemoryStore {}

impl StoragePathIntrospection for MemoryStore {}

impl StorageStatsIntrospection for MemoryStore {
    fn stats(&self) -> EngineStats {
        let mut out = self.stats;
        out.cache_evictions = self.cache_evictions;
        out
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use crate::key_codec::KeyEncoding;
    use crate::storage::{KvEngine, RangeReadEngine, TtlEngine, TtlState};

    use super::MemoryStore;

    #[test]
    fn key_expires_after_ttl() {
        let mut store = MemoryStore::new();
        let key = KeyEncoding::Int(1);
        store.set(key.clone(), b"v".to_vec()).expect("set");
        assert!(store.expire(&key, 1).expect("expire"));

        thread::sleep(Duration::from_millis(1100));

        assert_eq!(store.get(&key), None);
        assert_eq!(store.ttl(&key).expect("ttl"), TtlState::NotFound);
    }

    #[test]
    fn range_returns_inclusive_sorted_pairs() {
        let mut store = MemoryStore::new();
        store.set(KeyEncoding::Int(3), b"v3".to_vec()).expect("set 3");
        store.set(KeyEncoding::Int(1), b"v1".to_vec()).expect("set 1");
        store.set(KeyEncoding::Int(2), b"v2".to_vec()).expect("set 2");

        let got = store
            .range(&KeyEncoding::Int(1), &KeyEncoding::Int(2))
            .expect("range");

        assert_eq!(
            got,
            vec![
                (KeyEncoding::Int(1), b"v1".to_vec()),
                (KeyEncoding::Int(2), b"v2".to_vec()),
            ]
        );
    }
}




