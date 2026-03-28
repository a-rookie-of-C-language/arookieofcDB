use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

#[derive(Default)]
struct LenCommand;

impl Command for LenCommand {
    fn name(&self) -> &'static str {
        "len"
    }

    fn usage(&self) -> &'static str {
        "len"
    }

    fn description(&self) -> &'static str {
        "show total key count"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, _args: &str) -> io::Result<CommandOutput> {
        Ok(CommandOutput::message(ctx.store.len().to_string()))
    }
}

crate::submit_command!(LenCommand);
