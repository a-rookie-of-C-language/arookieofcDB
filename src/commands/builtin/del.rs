use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

use super::utils::{first_arg, parse_key};

#[derive(Default)]
struct DelCommand;

impl Command for DelCommand {
    fn name(&self) -> &'static str { "del" }
    fn usage(&self) -> &'static str { "del <key>" }
    fn description(&self) -> &'static str { "delete key" }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let key = parse_key(first_arg(args), "missing key for del")?;
        let deleted = ctx.kv().delete(&key)?;
        let removed = if deleted.is_some() { "1" } else { "0" };
        Ok(CommandOutput::message(removed))
    }
}

crate::submit_command!(DelCommand);
