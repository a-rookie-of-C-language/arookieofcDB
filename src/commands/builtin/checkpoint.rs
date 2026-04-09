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
        "write snapshot and compact wal"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        if !args.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "usage: checkpoint",
            ));
        }

        ctx.durability().checkpoint()?;
        Ok(CommandOutput::message("ok"))
    }
}

crate::submit_command!(CheckpointCommand);
