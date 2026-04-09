use super::kv_engine::KvEngine;
use super::disk_read_engine::DiskReadEngine;
use super::range_read_engine::RangeReadEngine;
use super::consistency_engine::ConsistencyEngine;
use super::consistency_repair_engine::ConsistencyRepairEngine;
use super::repair_control_engine::RepairControlEngine;
use super::fault_injection_engine::FaultInjectionEngine;
use super::ttl_engine::TtlEngine;
use super::durability_engine::DurabilityEngine;
use super::cache_config_engine::CacheConfigEngine;
use super::storage_introspection::StorageIntrospection;

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