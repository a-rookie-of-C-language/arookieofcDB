use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};
use crate::value_codec::StringEncoding;

use super::utils::{invalid_input, parse_key};

#[derive(Default)]
struct AddCommand;

impl Command for AddCommand {
    fn name(&self) -> &'static str { "add" }
    fn usage(&self) -> &'static str { "add <key> <value>" }
    fn description(&self) -> &'static str { "insert a new key-value pair (sql-like insert)" }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mut parts = args.trim_start().splitn(2, char::is_whitespace);
        let key = parse_key(parts.next(), "missing key for add")?;
        let value = parts.next().map(str::trim_start).ok_or_else(|| invalid_input("missing value for add"))?;

        if ctx.store.get(&key).is_some() {
            return Err(invalid_input("key already exists"));
        }

        let encoded = StringEncoding::from_input(value).encode();
        ctx.store.set(key, encoded)?;
        Ok(CommandOutput::message("1"))
    }
}

crate::submit_command!(AddCommand);
