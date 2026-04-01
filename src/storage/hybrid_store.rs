use std::collections::HashMap;
use std::io;
use std::path::Path;

use crate::key_codec::KeyEncoding;

use super::{
    CacheCapacityEngine, CachePolicyEngine, ConsistencyEngine, ConsistencyRepairEngine,
    DiskReadEngine, EngineStats, FaultInjectionEngine, FaultTarget, KvEngine, MemoryStore,
    RangeReadEngine, RepairControlEngine, RepairMode, RepairSummary,
    StoragePathIntrospection, StorageStatsIntrospection, TtlEngine, TtlState, WalStore,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CachePolicy {
    Lru,
    None,
}

impl CachePolicy {
    fn from_input(raw: &str) -> Option<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "lru" => Some(Self::Lru),
            "none" => Some(Self::None),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Lru => "lru",
            Self::None => "none",
        }
    }
}

#[derive(Debug)]
pub struct HybridStore {
    memory: MemoryStore,
    wal: WalStore,
    stats: EngineStats,
    strict_read_check: bool,
    repair_mode: RepairMode,
    cache_policy: CachePolicy,
    last_repair_summary: Option<RepairSummary>,
}

impl HybridStore {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let strict_read_check = std::env::var("AROOKIE_HYBRID_STRICT_READ")
            .ok()
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false);

        Self::open_with_mode(path, strict_read_check, if strict_read_check { RepairMode::Read } else { RepairMode::Off }, CachePolicy::Lru)
    }

    fn open_with_mode(
        path: impl AsRef<Path>,
        strict_read_check: bool,
    repair_mode: RepairMode,
    cache_policy: CachePolicy,
    ) -> io::Result<Self> {
        Ok(Self {
            memory: MemoryStore::new(),
            wal: WalStore::open(path)?,
            stats: EngineStats::default(),
            strict_read_check,
            repair_mode,
            cache_policy,
            last_repair_summary: None,
        })
    }

    #[cfg(test)]
    fn open_with_strict(path: impl AsRef<Path>, strict_read_check: bool) -> io::Result<Self> {
        Self::open_with_mode(path, strict_read_check, if strict_read_check { RepairMode::Read } else { RepairMode::Off }, CachePolicy::Lru)
    }

    fn read_repair_enabled(&self) -> bool {
        self.strict_read_check || matches!(self.repair_mode, RepairMode::Read | RepairMode::Always)
    }

    fn write_repair_enabled(&self) -> bool {
        matches!(self.repair_mode, RepairMode::Write | RepairMode::Always)
    }

    fn maybe_auto_repair_on_write(&mut self) -> io::Result<()> {
        if !self.write_repair_enabled() {
            return Ok(());
        }

        // Keep disk as source of truth and reconcile cache before write.
        let report = self.run_repair_consistency(super::RepairTarget::Cache)?;
        let repaired = report.total_repairs() as u64;
        self.stats.auto_repairs += repaired;
        self.stats.auto_repairs_write += repaired;
        Ok(())
    }

    fn run_repair_consistency(
        &mut self,
        target: super::RepairTarget,
    ) -> io::Result<super::RepairReport> {
        let (cache, disk) = self.snapshot_maps();
        let mut report = super::RepairReport {
            target,
            repaired_only_in_cache: 0,
            repaired_only_in_disk: 0,
            repaired_value_mismatches: 0,
        };

        match target {
            super::RepairTarget::Disk => {
                for (key, cache_value) in &cache {
                    match disk.get(key) {
                        None => {
                            self.wal.set(key.clone(), cache_value.clone())?;
                            report.repaired_only_in_cache += 1;
                        }
                        Some(disk_value) if disk_value != cache_value => {
                            self.wal.set(key.clone(), cache_value.clone())?;
                            report.repaired_value_mismatches += 1;
                        }
                        _ => {}
                    }
                }

                for key in disk.keys() {
                    if !cache.contains_key(key) {
                        let _ = self.wal.delete(key)?;
                        report.repaired_only_in_disk += 1;
                    }
                }
            }
            super::RepairTarget::Cache => {
                for (key, disk_value) in &disk {
                    match cache.get(key) {
                        None => {
                            self.memory.set(key.clone(), disk_value.clone())?;
                            report.repaired_only_in_disk += 1;
                        }
                        Some(cache_value) if cache_value != disk_value => {
                            self.memory.set(key.clone(), disk_value.clone())?;
                            report.repaired_value_mismatches += 1;
                        }
                        _ => {}
                    }
                }

                for key in cache.keys() {
                    if !disk.contains_key(key) {
                        let _ = self.memory.delete(key)?;
                        report.repaired_only_in_cache += 1;
                    }
                }
            }
        }

        self.last_repair_summary = Some(RepairSummary {
            target: report.target,
            repaired_only_in_cache: report.repaired_only_in_cache,
            repaired_only_in_disk: report.repaired_only_in_disk,
            repaired_value_mismatches: report.repaired_value_mismatches,
        });

        Ok(report)
    }

    pub fn sync(&mut self) -> io::Result<()> {
        self.wal.sync()
    }

    fn snapshot_maps(&mut self) -> (HashMap<KeyEncoding, Vec<u8>>, HashMap<KeyEncoding, Vec<u8>>) {
        let cache = self.memory.entries().into_iter().collect::<HashMap<_, _>>();
        let disk = self.wal.entries().into_iter().collect::<HashMap<_, _>>();
        (cache, disk)
    }

    fn build_consistency_report(
        cache: &HashMap<KeyEncoding, Vec<u8>>,
        disk: &HashMap<KeyEncoding, Vec<u8>>,
    ) -> super::ConsistencyReport {
        let mut only_in_cache = 0usize;
        let mut only_in_disk = 0usize;
        let mut value_mismatches = 0usize;
        let mut samples = Vec::new();

        for (key, cache_value) in cache {
            match disk.get(key) {
                None => {
                    only_in_cache += 1;
                    samples.push(super::ConsistencyDiff {
                        key: key.clone(),
                        kind: super::ConsistencyDiffKind::OnlyInCache,
                    });
                }
                Some(disk_value) if disk_value != cache_value => {
                    value_mismatches += 1;
                    samples.push(super::ConsistencyDiff {
                        key: key.clone(),
                        kind: super::ConsistencyDiffKind::ValueMismatch,
                    });
                }
                _ => {}
            }
        }

        for key in disk.keys() {
            if !cache.contains_key(key) {
                only_in_disk += 1;
                samples.push(super::ConsistencyDiff {
                    key: key.clone(),
                    kind: super::ConsistencyDiffKind::OnlyInDisk,
                });
            }
        }

        samples.sort_by(|a, b| a.key.cmp(&b.key));
        samples.truncate(10);

        super::ConsistencyReport {
            cache_keys: cache.len(),
            disk_keys: disk.len(),
            only_in_cache,
            only_in_disk,
            value_mismatches,
            samples,
        }
    }
}

impl KvEngine for HybridStore {
    fn engine_name(&self) -> &'static str {
        "hybrid"
    }

    fn len(&mut self) -> usize {
        self.wal.len()
    }

    fn get(&mut self, key: &KeyEncoding) -> Option<&[u8]> {
        self.stats.reads += 1;

        if self.cache_policy == CachePolicy::None {
            let (disk_ref, disk_expired) = self.wal.get_with_expiry_ref(key);
            if disk_expired {
                self.stats.ttl_expired_on_disk += 1;
            }
            if disk_ref.is_some() {
                self.stats.disk_reads += 1;
            }
            return disk_ref;
        }

        let (mem_value, mem_expired) = self.memory.get_with_expiry_flag(key);
        if mem_expired {
            self.stats.ttl_expired_in_cache += 1;
        }

        if let Some(mem_value) = mem_value {
            self.stats.cache_hits += 1;

            if self.read_repair_enabled() {
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
                            self.stats.auto_repairs += 1;
                            self.stats.auto_repairs_read += 1;
                        }
                        return self.memory.get(key);
                    }
                    None => {
                        let _ = self.memory.delete(key);
                        self.stats.cache_invalidated += 1;
                        self.stats.auto_repairs += 1;
                        self.stats.auto_repairs_read += 1;
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

    fn set(&mut self, key: KeyEncoding, value: Vec<u8>) -> io::Result<()> {
        self.maybe_auto_repair_on_write()?;
        self.stats.writes += 1;
        self.stats.disk_writes += 1;
        self.wal.set(key.clone(), value.clone())?;

        if self.cache_policy == CachePolicy::Lru {
            self.memory.set(key, value)
        } else {
            let _ = self.memory.delete(&key);
            Ok(())
        }
    }

    fn delete(&mut self, key: &KeyEncoding) -> io::Result<Option<Vec<u8>>> {
        self.maybe_auto_repair_on_write()?;
        self.stats.deletes += 1;
        let disk_deleted = self.wal.delete(key)?;
        let mem_deleted = self.memory.delete(key)?;
        if disk_deleted.is_some() {
            self.stats.disk_writes += 1;
        }
        Ok(disk_deleted.or(mem_deleted))
    }
}

impl DiskReadEngine for HybridStore {
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
}

impl RangeReadEngine for HybridStore {
    fn range(
        &mut self,
        start: &KeyEncoding,
        end: &KeyEncoding,
    ) -> io::Result<Vec<(KeyEncoding, Vec<u8>)>> {
        self.wal.range(start, end)
    }
}

impl ConsistencyEngine for HybridStore {
    fn verify_consistency(&mut self) -> io::Result<super::ConsistencyReport> {
        let (cache, disk) = self.snapshot_maps();
        Ok(Self::build_consistency_report(&cache, &disk))
    }
}

impl ConsistencyRepairEngine for HybridStore {
    fn repair_consistency(
        &mut self,
        target: super::RepairTarget,
    ) -> io::Result<super::RepairReport> {
        self.run_repair_consistency(target)
    }

    fn last_repair_summary(&self) -> Option<RepairSummary> {
        self.last_repair_summary
    }
}

impl RepairControlEngine for HybridStore {
    fn set_repair_mode(&mut self, mode: RepairMode) -> io::Result<()> {
        self.repair_mode = mode;
        Ok(())
    }

    fn repair_mode(&self) -> Option<RepairMode> {
        Some(self.repair_mode)
    }
}

impl FaultInjectionEngine for HybridStore {
    fn inject_fault(
        &mut self,
        target: FaultTarget,
        key: KeyEncoding,
        value: Vec<u8>,
    ) -> io::Result<()> {
        match target {
            FaultTarget::CacheOnly => self.memory.set(key, value),
            FaultTarget::DiskOnly => self.wal.set(key, value),
        }
    }
}

impl TtlEngine for HybridStore {
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
}

impl CacheCapacityEngine for HybridStore {
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
}

impl CachePolicyEngine for HybridStore {
    fn set_cache_policy(&mut self, policy: &str) -> io::Result<()> {
        let Some(policy) = CachePolicy::from_input(policy) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid cache policy: use lru|none",
            ));
        };

        self.cache_policy = policy;
        if self.cache_policy == CachePolicy::None {
            self.memory.clear_cache();
        }
        Ok(())
    }

    fn cache_policy(&self) -> Option<&'static str> {
        Some(self.cache_policy.as_str())
    }
}

impl StoragePathIntrospection for HybridStore {
    fn wal_path(&self) -> Option<&Path> {
        self.wal.wal_path().into()
    }

    fn snapshot_path(&self) -> Option<&Path> {
        self.wal.snapshot_path().into()
    }
}

impl StorageStatsIntrospection for HybridStore {
    fn stats(&self) -> EngineStats {
        let mut out = self.stats;
        let wal_stats = self.wal.stats();
        out.wal_appends = wal_stats.wal_appends;
        out.fsync_count = wal_stats.fsync_count;
        out.ttl_loaded_on_startup = wal_stats.ttl_loaded_on_startup;
        out.ttl_pruned_on_startup = wal_stats.ttl_pruned_on_startup;
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
    use crate::storage::{
        CacheCapacityEngine, CachePolicyEngine, ConsistencyEngine,
        ConsistencyRepairEngine, FaultInjectionEngine, KvEngine, RangeReadEngine,
        RepairControlEngine, StorageStatsIntrospection, TtlEngine,
    };

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

    #[test]
    fn cache_policy_none_disables_backfill() {
        let wal_path = unique_wal_path();
        let mut store = HybridStore::open(&wal_path).expect("open hybrid");
        let key = KeyEncoding::Raw("p:none".to_string());

        store.set_cache_policy("none").expect("set policy none");
        store.set(key.clone(), b"v".to_vec()).expect("set");

        assert_eq!(store.cache_current_keys(), Some(0));
        assert_eq!(store.get(&key), Some("v".as_bytes()));
        assert_eq!(store.cache_current_keys(), Some(0));

        let stats_after_none = store.stats();
        let hits_before = stats_after_none.cache_hits;

        store.set_cache_policy("lru").expect("set policy lru");
        assert_eq!(store.get(&key), Some("v".as_bytes()));
        assert_eq!(store.cache_current_keys(), Some(1));

        let stats_after_lru = store.stats();
        assert!(stats_after_lru.cache_hits >= hits_before);

        cleanup(&wal_path);
    }    #[test]
    fn verify_detects_mismatch_and_repair_to_disk() {
        let wal_path = unique_wal_path();
        let key = KeyEncoding::Raw("vr:key".to_string());

        let mut store = HybridStore::open(&wal_path).expect("open hybrid");
        store.set(key.clone(), b"disk".to_vec()).expect("set");
        let _ = store.memory.set(key.clone(), b"cache".to_vec());

        let report = store.verify_consistency().expect("verify");
        assert_eq!(report.value_mismatches, 1);

        let repaired = store
            .repair_consistency(super::super::RepairTarget::Disk)
            .expect("repair");
        assert_eq!(repaired.repaired_value_mismatches, 1);

        let report_after = store.verify_consistency().expect("verify after");
        assert_eq!(report_after.total_issues(), 0);

        cleanup(&wal_path);
    }

    #[test]
    fn verify_detects_disk_only_and_repair_to_cache() {
        let wal_path = unique_wal_path();
        let key = KeyEncoding::Raw("vr:disk-only".to_string());

        let mut store = HybridStore::open(&wal_path).expect("open hybrid");
        store.set(key.clone(), b"v".to_vec()).expect("set");
        let _ = store.memory.delete(&key);

        let report = store.verify_consistency().expect("verify");
        assert_eq!(report.only_in_disk, 1);

        let repaired = store
            .repair_consistency(super::super::RepairTarget::Cache)
            .expect("repair");
        assert_eq!(repaired.repaired_only_in_disk, 1);

        let report_after = store.verify_consistency().expect("verify after");
        assert_eq!(report_after.total_issues(), 0);

        cleanup(&wal_path);
    }    #[test]
    fn fault_cache_only_creates_only_in_cache_issue() {
        let wal_path = unique_wal_path();
        let key = KeyEncoding::Raw("fault:cache".to_string());

        let mut store = HybridStore::open(&wal_path).expect("open hybrid");
        store
            .inject_fault(
                super::super::FaultTarget::CacheOnly,
                key.clone(),
                b"v".to_vec(),
            )
            .expect("inject cache-only");

        let report = store.verify_consistency().expect("verify");
        assert_eq!(report.only_in_cache, 1);

        cleanup(&wal_path);
    }

    #[test]
    fn fault_disk_only_creates_only_in_disk_issue() {
        let wal_path = unique_wal_path();
        let key = KeyEncoding::Raw("fault:disk".to_string());

        let mut store = HybridStore::open(&wal_path).expect("open hybrid");
        store
            .inject_fault(
                super::super::FaultTarget::DiskOnly,
                key.clone(),
                b"v".to_vec(),
            )
            .expect("inject disk-only");

        let report = store.verify_consistency().expect("verify");
        assert_eq!(report.only_in_disk, 1);

        cleanup(&wal_path);
    }    #[test]
    fn repair_mode_read_enables_auto_repair() {
        let wal_path = unique_wal_path();
        let key = KeyEncoding::Raw("rm:key".to_string());

        let mut store = HybridStore::open_with_strict(&wal_path, false).expect("open hybrid");
        store.set(key.clone(), b"disk".to_vec()).expect("set");
        store
            .inject_fault(super::super::FaultTarget::CacheOnly, key.clone(), b"stale".to_vec())
            .expect("inject");

        assert_eq!(store.get(&key), Some("stale".as_bytes()));
        assert!(store.verify_consistency().expect("verify off").total_issues() >= 1);

        store
            .set_repair_mode(super::super::RepairMode::Read)
            .expect("set repair mode");

        assert_eq!(store.get(&key), Some("disk".as_bytes()));
        assert_eq!(store.verify_consistency().expect("verify read").total_issues(), 0);
        assert!(store.stats().auto_repairs >= 1);

        cleanup(&wal_path);
    }
    #[test]
    fn repair_mode_write_repairs_on_next_write() {
        let wal_path = unique_wal_path();
        let ghost = KeyEncoding::Raw("rm:write:ghost".to_string());
        let trigger = KeyEncoding::Raw("rm:write:trigger".to_string());

        let mut store = HybridStore::open_with_strict(&wal_path, false).expect("open hybrid");

        store
            .inject_fault(super::super::FaultTarget::DiskOnly, ghost.clone(), b"v".to_vec())
            .expect("inject disk-only");
        assert_eq!(store.verify_consistency().expect("verify before").only_in_disk, 1);

        store
            .set_repair_mode(super::super::RepairMode::Write)
            .expect("set repair mode write");

        store.set(trigger, b"x".to_vec()).expect("trigger write");

        let report_after = store.verify_consistency().expect("verify after");
        assert_eq!(report_after.total_issues(), 0, "report={report_after:?}");
        assert!(store.stats().auto_repairs >= 1);

        cleanup(&wal_path);
    }

    #[test]
    fn range_reads_disk_truth_in_sorted_order() {
        let wal_path = unique_wal_path();
        let mut store = HybridStore::open(&wal_path).expect("open hybrid");

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

        cleanup(&wal_path);
    }
}








