use super::engine_stats::EngineStats;

pub trait StorageStatsIntrospection {
    fn stats(&self) -> EngineStats {
        EngineStats::default()
    }
}