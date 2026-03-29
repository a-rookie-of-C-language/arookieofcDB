use std::io;
use std::path::Path;

use crate::key_codec::KeyEncoding;

use super::{EngineStats, MemoryStore, StorageEngine, SyncPolicy, TtlState, WalStore};

#[derive(Debug)]
pub struct HybridStore {
    memory: MemoryStore,
    wal: WalStore,
    stats: EngineStats,
    strict_read_check: bool,
}

impl HybridStore {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let strict_read_check = std::env::var("AROOKIE_HYBRID_STRICT_READ")
            .ok()
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false);

        Self::open_with_mode(path, strict_read_check)
    }

    fn open_with_mode(path: impl AsRef<Path>, strict_read_check: bool) -> io::Result<Self> {
        Ok(Self {
            memory: MemoryStore::new(),
            wal: WalStore::open(path)?,
            stats: EngineStats::default(),
            strict_read_check,
        })
    }

    #[cfg(test)]
    fn open_with_strict(path: impl AsRef<Path>, strict_read_check: bool) -> io::Result<Self> {
        Self::open_with_mode(path, strict_read_check)
    }
}

impl StorageEngine for HybridStore {
    fn engine_name(&self) -> &'static str {
        "hybrid"
    }

    fn len(&mut self) -> usize {
        self.wal.len()
    }

    fn get(&mut self, key: &KeyEncoding) -> Option<&[u8]> {
        self.stats.reads += 1;

        let (mem_value, mem_expired) = self.memory.get_with_expiry_flag(key);
        if mem_expired {
            self.stats.ttl_expired_in_cache += 1;
        }

        if let Some(mem_value) = mem_value {
            self.stats.cache_hits += 1;

            if self.strict_read_check {
                self.stats.disk_reads += 1;
                let (disk_value, disk_expired) = self.wal.get_with_expiry_flag(key);
                if disk_expired {
                    self.stats.ttl_expired_on_disk += 1;
                }

                match disk_value {
                    Some(disk_value) => {
                        if disk_value != mem_value {
                            let _ = self.memory.set(key.clone(), disk_value);
                            self.stats.cache_repaired += 1;
                        }
                        return self.memory.get(key);
                    }
                    None => {
                        let _ = self.memory.delete(key);
                        self.stats.cache_invalidated += 1;
                        return None;
                    }
                }
            }

            return self.memory.get(key);
        }

        self.stats.cache_misses += 1;

        let (disk_value, disk_expired) = self.wal.get_with_expiry_flag(key);
        if disk_expired {
            self.stats.ttl_expired_on_disk += 1;
        }

        if let Some(v) = disk_value {
            self.stats.disk_reads += 1;
            let _ = self.memory.set(key.clone(), v);
            return self.memory.get(key);
        }

        None
    }

    fn get_disk_only(&mut self, key: &KeyEncoding) -> Option<Vec<u8>> {
        self.stats.reads += 1;
        let (disk_value, disk_expired) = self.wal.get_with_expiry_flag(key);
        if disk_expired {
            self.stats.ttl_expired_on_disk += 1;
        }
        if disk_value.is_some() {
            self.stats.disk_reads += 1;
        }
        disk_value
    }

    fn set(&mut self, key: KeyEncoding, value: Vec<u8>) -> io::Result<()> {
        self.stats.writes += 1;
        self.stats.disk_writes += 1;
        self.wal.set(key.clone(), value.clone())?;
        self.memory.set(key, value)
    }

    fn delete(&mut self, key: &KeyEncoding) -> io::Result<Option<Vec<u8>>> {
        self.stats.deletes += 1;
        let disk_deleted = self.wal.delete(key)?;
        let mem_deleted = self.memory.delete(key)?;
        if disk_deleted.is_some() {
            self.stats.disk_writes += 1;
        }
        Ok(disk_deleted.or(mem_deleted))
    }

    fn range_query(
        &self,
        start: &KeyEncoding,
        end: &KeyEncoding,
    ) -> Vec<(KeyEncoding, Vec<u8>)> {
        self.wal.range_query(start, end)
    }

    fn expire(&mut self, key: &KeyEncoding, seconds: u64) -> io::Result<bool> {
        let changed = self.wal.expire(key, seconds)?;
        if changed {
            let _ = self.memory.expire(key, seconds)?;
        }
        Ok(changed)
    }

    fn ttl(&mut self, key: &KeyEncoding) -> io::Result<TtlState> {
        self.wal.ttl(key)
    }

    fn sync(&mut self) -> io::Result<()> {
        self.wal.sync()
    }

    fn checkpoint(&mut self) -> io::Result<()> {
        self.wal.checkpoint()
    }

    fn sync_policy(&self) -> SyncPolicy {
        self.wal.sync_policy()
    }

    fn set_sync_policy(&mut self, policy: SyncPolicy) {
        self.wal.set_sync_policy(policy);
        self.memory.set_sync_policy(policy);
    }

    fn set_cache_max_keys(&mut self, max_keys: usize) -> io::Result<()> {
        self.memory.set_cache_max_keys(max_keys);
        Ok(())
    }

    fn cache_max_keys(&self) -> Option<usize> {
        Some(self.memory.cache_max_keys())
    }

    fn cache_current_keys(&self) -> Option<usize> {
        Some(self.memory.cache_current_keys())
    }

    fn wal_path(&self) -> Option<&Path> {
        self.wal.wal_path().into()
    }

    fn snapshot_path(&self) -> Option<&Path> {
        self.wal.snapshot_path().into()
    }

    fn stats(&self) -> EngineStats {
        let mut out = self.stats;
        let wal_stats = self.wal.stats();
        out.wal_appends = wal_stats.wal_appends;
        out.fsync_count = wal_stats.fsync_count;
        out.cache_evictions = self.memory.cache_evictions();
        out
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use crate::key_codec::KeyEncoding;
    use crate::storage::StorageEngine;

    use super::HybridStore;

    fn unique_wal_path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!("arookieofcdb-hybrid-test-{nanos}.wal"))
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("snap"));
        let _ = std::fs::remove_file(path.with_extension("snap.tmp"));
    }

    #[test]
    fn hybrid_ttl_expires_key() {
        let wal_path = unique_wal_path();
        let mut store = HybridStore::open(&wal_path).expect("open hybrid");
        let key = KeyEncoding::Int(7);

        store.set(key.clone(), b"v7".to_vec()).expect("set");
        assert!(store.expire(&key, 1).expect("expire"));

        thread::sleep(Duration::from_millis(1100));

        assert_eq!(store.get(&key), None);

        cleanup(&wal_path);
    }

    #[test]
    fn hybrid_exposes_ttl_expired_counters() {
        let wal_path = unique_wal_path();
        let mut store = HybridStore::open(&wal_path).expect("open hybrid");
        let key = KeyEncoding::Raw("ttl:counter".to_string());

        store.set(key.clone(), b"v".to_vec()).expect("set");
        assert!(store.expire(&key, 1).expect("expire"));

        thread::sleep(Duration::from_millis(1100));

        assert_eq!(store.get(&key), None);
        let stats = store.stats();
        assert!(stats.ttl_expired_in_cache >= 1);
        assert!(stats.ttl_expired_on_disk >= 1);

        cleanup(&wal_path);
    }

    #[test]
    fn strict_read_repair_fixes_stale_cache() {
        let wal_path = unique_wal_path();
        let key = KeyEncoding::Raw("repair:key".to_string());

        let mut store = HybridStore::open_with_strict(&wal_path, true).expect("open strict hybrid");
        store.set(key.clone(), b"fresh".to_vec()).expect("set fresh");

        let _ = store.memory.set(key.clone(), b"stale".to_vec());

        assert_eq!(store.get(&key), Some("fresh".as_bytes()));

        let stats = store.stats();
        assert!(stats.cache_repaired >= 1);

        cleanup(&wal_path);
    }

    #[test]
    fn strict_read_repair_invalidates_ghost_cache() {
        let wal_path = unique_wal_path();
        let key = KeyEncoding::Raw("ghost:key".to_string());

        let mut store = HybridStore::open_with_strict(&wal_path, true).expect("open strict hybrid");
        store.set(key.clone(), b"v".to_vec()).expect("set");

        let _ = store.wal.delete(&key);

        assert_eq!(store.get(&key), None);

        let stats = store.stats();
        assert!(stats.cache_invalidated >= 1);

        cleanup(&wal_path);
    }

    #[test]
    fn cache_limit_evicts_only_memory_and_can_backfill() {
        let wal_path = unique_wal_path();
        let mut store = HybridStore::open(&wal_path).expect("open hybrid");
        store.set_cache_max_keys(1).expect("set limit");

        let k1 = KeyEncoding::Raw("k1".to_string());
        let k2 = KeyEncoding::Raw("k2".to_string());

        store.set(k1.clone(), b"v1".to_vec()).expect("set k1");
        store.set(k2.clone(), b"v2".to_vec()).expect("set k2");

        assert_eq!(store.cache_current_keys(), Some(1));
        assert!(store.stats().cache_evictions >= 1);

        assert_eq!(store.get(&k1), Some("v1".as_bytes()));

        cleanup(&wal_path);
    }
}

