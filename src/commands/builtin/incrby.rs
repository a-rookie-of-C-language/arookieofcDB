use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};
use crate::key_codec::KeyEncoding;
use crate::value_codec::StringEncoding;

use super::utils::{invalid_input, parse_i64, parse_key};

#[derive(Default)]
struct IncrByCommand;

impl Command for IncrByCommand {
    fn name(&self) -> &'static str { "incrby" }
    fn usage(&self) -> &'static str { "incrby <key> <increment>" }
    fn description(&self) -> &'static str { "increment integer value by a given number" }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mut parts = args.split_whitespace();
        let key = parse_key(parts.next(), "missing key for incrby")?;
        let delta = parse_i64(parts.next(), "missing increment for incrby")?;

        let current = current_i64(ctx, &key)?;
        let next = current.checked_add(delta).ok_or_else(|| invalid_input("integer overflow"))?;

        ctx.kv().set(key, StringEncoding::Int(next).encode())?;
        Ok(CommandOutput::message(next.to_string()))
    }
}

fn current_i64(ctx: &mut CommandContext<'_>, key: &KeyEncoding) -> io::Result<i64> {
    let Some(raw) = ctx.kv().get(key) else { return Ok(0); };

    match StringEncoding::decode(&raw) {
        StringEncoding::Int(v) => Ok(v),
        StringEncoding::Raw(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            text.parse::<i64>().map_err(|_| invalid_input("value is not an integer"))
        }
        StringEncoding::Float(_) => Err(invalid_input("value is not an integer")),
    }
}

crate::submit_command!(IncrByCommand);
