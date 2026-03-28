use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

#[derive(Default)]
struct CheckpointCommand;

impl Command for CheckpointCommand {
    fn name(&self) -> &'static str {
        "checkpoint"
    }

    fn usage(&self) -> &'static str {
        "checkpoint"
    }

    fn description(&self) -> &'static str {
        "flush snapshot and rotate wal"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, _args: &str) -> io::Result<CommandOutput> {
        ctx.store.checkpoint()?;
        Ok(CommandOutput::message("ok (checkpoint created)"))
    }
}

crate::submit_command!(CheckpointCommand);
