use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

use super::utils::file_size_or_zero_opt;

#[derive(Default)]
struct CompactCommand;

impl Command for CompactCommand {
    fn name(&self) -> &'static str {
        "compact"
    }

    fn usage(&self) -> &'static str {
        "compact"
    }

    fn description(&self) -> &'static str {
        "compact wal using checkpoint"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, _args: &str) -> io::Result<CommandOutput> {
        let before = file_size_or_zero_opt(ctx.store.wal_path());
        ctx.store.checkpoint()?;
        let after = file_size_or_zero_opt(ctx.store.wal_path());
        Ok(CommandOutput::message(format!(
            "ok (compact done, wal bytes: {before} -> {after})"
        )))
    }
}

crate::submit_command!(CompactCommand);
