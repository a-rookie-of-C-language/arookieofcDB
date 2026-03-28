use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

#[derive(Default)]
struct SyncCommand;

impl Command for SyncCommand {
    fn name(&self) -> &'static str {
        "sync"
    }

    fn usage(&self) -> &'static str {
        "sync"
    }

    fn description(&self) -> &'static str {
        "force fsync wal"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, _args: &str) -> io::Result<CommandOutput> {
        ctx.store.sync()?;
        Ok(CommandOutput::message("ok"))
    }
}

crate::submit_command!(SyncCommand);
