use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

use super::utils::{first_arg, parse_i64};

#[derive(Default)]
struct DelCommand;

impl Command for DelCommand {
    fn name(&self) -> &'static str {
        "del"
    }

    fn usage(&self) -> &'static str {
        "del <key>"
    }

    fn description(&self) -> &'static str {
        "delete key"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let key = parse_i64(first_arg(args), "missing key for del")?;
        let deleted = ctx.store.delete(key)?;
        let removed = if deleted.is_some() { "1" } else { "0" };
        Ok(CommandOutput::message(removed))
    }
}

crate::submit_command!(DelCommand);
