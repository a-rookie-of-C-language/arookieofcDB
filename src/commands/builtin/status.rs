use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};
use crate::storage::SyncPolicy;

use super::utils::{file_mtime_unix_opt, file_size_or_zero_opt};

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
        let mode = match ctx.store.sync_policy() {
            SyncPolicy::Always => "always",
            SyncPolicy::Manual => "manual",
        };

        let wal_bytes = file_size_or_zero_opt(ctx.store.wal_path());
        let snapshot_bytes = file_size_or_zero_opt(ctx.store.snapshot_path());
        let snapshot_mtime = file_mtime_unix_opt(ctx.store.snapshot_path())
            .map(|ts| ts.to_string())
            .unwrap_or_else(|| String::from("none"));
        let wal_path = ctx
            .store
            .wal_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| String::from("none"));
        let snapshot_path = ctx
            .store
            .snapshot_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| String::from("none"));

        let stats = ctx.store.stats();
        let cache_max_keys = ctx
            .store
            .cache_max_keys()
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from("none"));
        let cache_current_keys = ctx
            .store
            .cache_current_keys()
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from("none"));
        let cache_policy = ctx
            .store
            .cache_policy()
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from("none"));

        Ok(CommandOutput::message(format!(
            "engine={}, len={}, syncmode={}, cache_policy={}, wal={}, wal_bytes={}, snapshot={}, snapshot_bytes={}, snapshot_mtime_unix={}, reads={}, writes={}, deletes={}, cache_hits={}, cache_misses={}, disk_reads={}, disk_writes={}, wal_appends={}, fsync_count={}, ttl_expired_in_cache={}, ttl_expired_on_disk={}, cache_repaired={}, cache_invalidated={}, cache_evictions={}, cache_max_keys={}, cache_current_keys={}",
            ctx.store.engine_name(),
            ctx.store.len(),
            mode,
            cache_policy,
            wal_path,
            wal_bytes,
            snapshot_path,
            snapshot_bytes,
            snapshot_mtime,
            stats.reads,
            stats.writes,
            stats.deletes,
            stats.cache_hits,
            stats.cache_misses,
            stats.disk_reads,
            stats.disk_writes,
            stats.wal_appends,
            stats.fsync_count,
            stats.ttl_expired_in_cache,
            stats.ttl_expired_on_disk,
            stats.cache_repaired,
            stats.cache_invalidated,
            stats.cache_evictions,
            cache_max_keys,
            cache_current_keys,
        )))
    }
}

crate::submit_command!(StatusCommand);

