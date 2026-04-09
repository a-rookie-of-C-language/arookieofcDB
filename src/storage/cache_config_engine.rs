use super::cache_capacity_engine::CacheCapacityEngine;
use super::cache_policy_engine::CachePolicyEngine;

pub trait CacheConfigEngine: CacheCapacityEngine + CachePolicyEngine {}
impl<T> CacheConfigEngine for T where T: CacheCapacityEngine + CachePolicyEngine {}