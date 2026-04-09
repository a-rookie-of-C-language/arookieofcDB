use std::io;
use crate::commands::builtin::utils::{file_mtime_unix_opt, file_size_or_zero_opt};
use crate::storage::{
    CacheCapacityEngine, CachePolicyEngine, ConsistencyEngine,
    ConsistencyRepairEngine, DiskReadEngine, DurabilityEngine,
    FaultInjectionEngine, KvEngine, RangeReadEngine, RepairControlEngine, RepairTarget,
    StorageEngine, StorageIntrospection, TtlEngine,
};
use super::status_snapshot::StatusSnapshot;

pub struct CommandContext<'a> {
    pub store: &'a mut dyn StorageEngine,
}

impl<'a> CommandContext<'a> {
    pub fn kv(&mut self) -> &mut dyn KvEngine {
        self.store
    }

    pub fn ttl(&mut self) -> &mut dyn TtlEngine {
        self.store
    }

    pub fn durability(&mut self) -> &mut dyn DurabilityEngine {
        self.store
    }

    pub fn disk_read(&mut self) -> &mut dyn DiskReadEngine {
        self.store
    }

    pub fn range_read(&mut self) -> &mut dyn RangeReadEngine {
        self.store
    }

    pub fn consistency(&mut self) -> &mut dyn ConsistencyEngine {
        self.store
    }

    pub fn repair(&mut self) -> &mut dyn ConsistencyRepairEngine {
        self.store
    }

    pub fn repair_mode(&mut self) -> &mut dyn RepairControlEngine {
        self.store
    }

    pub fn fault(&mut self) -> &mut dyn FaultInjectionEngine {
        self.store
    }

    pub fn cache_limits(&mut self) -> &mut dyn CacheCapacityEngine {
        self.store
    }

    pub fn cache_config(&mut self) -> &mut dyn CachePolicyEngine {
        self.store
    }

    pub fn inspect(&mut self) -> &dyn StorageIntrospection {
        self.store
    }

    pub fn status_report(&mut self) -> io::Result<StatusSnapshot> {
        let introspection = self.inspect();
        let wal_bytes = file_size_or_zero_opt(introspection.wal_path());
        let snapshot_bytes = file_size_or_zero_opt(introspection.snapshot_path());
        let snapshot_mtime_unix = file_mtime_unix_opt(introspection.snapshot_path())
            .map(|ts: u64| ts.to_string())
            .unwrap_or_else(|| String::from("none"));
        let wal_path = introspection
            .wal_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| String::from("none"));
        let snapshot_path = introspection
            .snapshot_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| String::from("none"));
        let stats = introspection.stats();

        let cache_limits = self.cache_limits();
        let cache_max_keys = cache_limits
            .cache_max_keys()
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from("none"));
        let cache_current_keys = cache_limits
            .cache_current_keys()
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from("none"));
        let cache_policy = self
            .cache_config()
            .cache_policy()
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from("none"));

        let repair_mode = self
            .repair_mode()
            .repair_mode()
            .map(|v| v.as_str().to_string())
            .unwrap_or_else(|| String::from("none"));

        let (
            inconsistency_total,
            inconsistency_only_in_cache,
            inconsistency_only_in_disk,
            inconsistency_value_mismatch,
        ) = match self.consistency().verify_consistency() {
            Ok(report) => (
                report.total_issues().to_string(),
                report.only_in_cache.to_string(),
                report.only_in_disk.to_string(),
                report.value_mismatches.to_string(),
            ),
            Err(err) if err.kind() == io::ErrorKind::Unsupported => (
                String::from("none"),
                String::from("none"),
                String::from("none"),
                String::from("none"),
            ),
            Err(err) => return Err(err),
        };

        let (
            last_repair_target,
            last_repair_total,
            last_repair_only_in_cache,
            last_repair_only_in_disk,
            last_repair_value_mismatch,
        ) = match self.repair().last_repair_summary() {
            Some(summary) => {
                let target = match summary.target {
                    RepairTarget::Disk => "disk",
                    RepairTarget::Cache => "cache",
                };
                (
                    target.to_string(),
                    summary.total_repairs().to_string(),
                    summary.repaired_only_in_cache.to_string(),
                    summary.repaired_only_in_disk.to_string(),
                    summary.repaired_value_mismatches.to_string(),
                )
            }
            None => (
                String::from("none"),
                String::from("none"),
                String::from("none"),
                String::from("none"),
                String::from("none"),
            ),
        };

        Ok(StatusSnapshot {
            engine: self.kv().engine_name(),
            len: self.kv().len(),
            stats,
            wal_path,
            wal_bytes,
            snapshot_path,
            snapshot_bytes,
            snapshot_mtime_unix,
            cache_policy,
            cache_max_keys,
            cache_current_keys,
            repair_mode,
            inconsistency_total,
            inconsistency_only_in_cache,
            inconsistency_only_in_disk,
            inconsistency_value_mismatch,
            last_repair_target,
            last_repair_total,
            last_repair_only_in_cache,
            last_repair_only_in_disk,
            last_repair_value_mismatch,
        })
    }

    pub fn cache_max_keys_string(&mut self) -> String {
        self.cache_limits()
            .cache_max_keys()
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from("unsupported"))
    }

    pub fn cache_policy_string(&mut self) -> String {
        self.cache_config()
            .cache_policy()
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from("unsupported"))
    }
}
