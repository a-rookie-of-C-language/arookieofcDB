use super::storage_path_introspection::StoragePathIntrospection;
use super::storage_stats_introspection::StorageStatsIntrospection;

pub trait StorageIntrospection: StoragePathIntrospection + StorageStatsIntrospection {}
impl<T> StorageIntrospection for T where T: StoragePathIntrospection + StorageStatsIntrospection {}