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
        "flush wal data to durable storage"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        if !args.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "usage: sync",
            ));
        }

        ctx.durability().sync()?;
        Ok(CommandOutput::message("ok"))
    }
}

crate::submit_command!(SyncCommand);
