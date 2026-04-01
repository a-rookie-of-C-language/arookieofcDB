use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

use super::utils::parse_key;

#[derive(Default)]
struct DeleteCommand;

impl Command for DeleteCommand {
    fn name(&self) -> &'static str { "delete" }
    fn usage(&self) -> &'static str { "delete <key>" }
    fn description(&self) -> &'static str { "delete key-value pair (sql-like delete)" }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mut parts = args.split_whitespace();
        let key = parse_key(parts.next(), "missing key for delete")?;
        let deleted = ctx.kv().delete(&key)?;
        Ok(CommandOutput::message(if deleted.is_some() { "1" } else { "0" }))
    }
}

crate::submit_command!(DeleteCommand);
