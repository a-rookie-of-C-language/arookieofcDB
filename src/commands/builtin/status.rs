use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

#[derive(Default)]
struct StatusCommand;

impl Command for StatusCommand {
    fn name(&self) -> &'static str {
        "status"
    }

    fn usage(&self) -> &'static str {
        "status"
    }

    fn description(&self) -> &'static str {
        "show store runtime status"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, _args: &str) -> io::Result<CommandOutput> {
        let snapshot = ctx.status_report()?;

        Ok(CommandOutput::message(format!(
            "engine={}, len={}, cache_policy={}, repair_mode={}, wal={}, wal_bytes={}, snapshot={}, snapshot_bytes={}, snapshot_mtime_unix={}, reads={}, writes={}, deletes={}, cache_hits={}, cache_misses={}, disk_reads={}, disk_writes={}, wal_appends={}, fsync_count={}, ttl_expired_in_cache={}, ttl_expired_on_disk={}, ttl_loaded_on_startup={}, ttl_pruned_on_startup={}, cache_repaired={}, cache_invalidated={}, cache_evictions={}, auto_repairs={}, auto_repairs_read={}, auto_repairs_write={}, cache_max_keys={}, cache_current_keys={}, inconsistency_total={}, inconsistency_only_in_cache={}, inconsistency_only_in_disk={}, inconsistency_value_mismatch={}, last_repair_target={}, last_repair_total={}, last_repair_only_in_cache={}, last_repair_only_in_disk={}, last_repair_value_mismatch={}",
            snapshot.engine,
            snapshot.len,
            snapshot.cache_policy,
            snapshot.repair_mode,
            snapshot.wal_path,
            snapshot.wal_bytes,
            snapshot.snapshot_path,
            snapshot.snapshot_bytes,
            snapshot.snapshot_mtime_unix,
            snapshot.stats.reads,
            snapshot.stats.writes,
            snapshot.stats.deletes,
            snapshot.stats.cache_hits,
            snapshot.stats.cache_misses,
            snapshot.stats.disk_reads,
            snapshot.stats.disk_writes,
            snapshot.stats.wal_appends,
            snapshot.stats.fsync_count,
            snapshot.stats.ttl_expired_in_cache,
            snapshot.stats.ttl_expired_on_disk,
            snapshot.stats.ttl_loaded_on_startup,
            snapshot.stats.ttl_pruned_on_startup,
            snapshot.stats.cache_repaired,
            snapshot.stats.cache_invalidated,
            snapshot.stats.cache_evictions,
            snapshot.stats.auto_repairs,
            snapshot.stats.auto_repairs_read,
            snapshot.stats.auto_repairs_write,
            snapshot.cache_max_keys,
            snapshot.cache_current_keys,
            snapshot.inconsistency_total,
            snapshot.inconsistency_only_in_cache,
            snapshot.inconsistency_only_in_disk,
            snapshot.inconsistency_value_mismatch,
            snapshot.last_repair_target,
            snapshot.last_repair_total,
            snapshot.last_repair_only_in_cache,
            snapshot.last_repair_only_in_disk,
            snapshot.last_repair_value_mismatch,
        )))
    }
}

crate::submit_command!(StatusCommand);





