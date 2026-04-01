use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};
use crate::value_codec::StringEncoding;

use super::utils::{invalid_input, parse_key};

#[derive(Default)]
struct UpdateCommand;

impl Command for UpdateCommand {
    fn name(&self) -> &'static str { "update" }
    fn usage(&self) -> &'static str { "update <key> <value>" }
    fn description(&self) -> &'static str { "update existing key-value pair (sql-like update)" }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mut parts = args.trim_start().splitn(2, char::is_whitespace);
        let key = parse_key(parts.next(), "missing key for update")?;
        let value = parts.next().map(str::trim_start).ok_or_else(|| invalid_input("missing value for update"))?;

        if ctx.kv().get(&key).is_none() {
            return Ok(CommandOutput::message("0"));
        }

        let encoded = StringEncoding::from_input(value).encode();
        ctx.kv().set(key, encoded)?;
        Ok(CommandOutput::message("1"))
    }
}

crate::submit_command!(UpdateCommand);
