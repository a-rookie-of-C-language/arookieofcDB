use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};
use super::utils::{invalid_input, parse_i64};

#[derive(Default)]
struct SetCommand;

impl Command for SetCommand {
    fn name(&self) -> &'static str {
        "set"
    }

    fn usage(&self) -> &'static str {
        "set <key> <value>"
    }

    fn description(&self) -> &'static str {
        "set value for a key"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mut parts = args.trim_start().splitn(2, char::is_whitespace);
        let key = parse_i64(parts.next(), "missing key for set")?;
        let value = parts
            .next()
            .map(str::trim_start)
            .ok_or_else(|| invalid_input("missing value for set"))?;

        ctx.store.set(key, value.as_bytes().to_vec())?;
        Ok(CommandOutput::message("ok"))
    }
}

crate::submit_command!(SetCommand);
