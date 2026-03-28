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

        Ok(CommandOutput::message(format!(
            "engine={}, len={}, syncmode={}, wal={}, wal_bytes={}, snapshot={}, snapshot_bytes={}, snapshot_mtime_unix={}",
            ctx.store.engine_name(),
            ctx.store.len(),
            mode,
            wal_path,
            wal_bytes,
            snapshot_path,
            snapshot_bytes,
            snapshot_mtime
        )))
    }
}

crate::submit_command!(StatusCommand);
