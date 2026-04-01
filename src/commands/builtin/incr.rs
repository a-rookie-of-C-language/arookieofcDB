use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};
use crate::key_codec::KeyEncoding;
use crate::value_codec::StringEncoding;

use super::utils::{first_arg, invalid_input, parse_key};

#[derive(Default)]
struct IncrCommand;

impl Command for IncrCommand {
    fn name(&self) -> &'static str { "incr" }
    fn usage(&self) -> &'static str { "incr <key>" }
    fn description(&self) -> &'static str { "increment integer value by 1" }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let key = parse_key(first_arg(args), "missing key for incr")?;
        let current = current_i64(ctx, &key)?;
        let next = current.checked_add(1).ok_or_else(|| invalid_input("integer overflow"))?;

        ctx.kv().set(key, StringEncoding::Int(next).encode())?;
        Ok(CommandOutput::message(next.to_string()))
    }
}

fn current_i64(ctx: &mut CommandContext<'_>, key: &KeyEncoding) -> io::Result<i64> {
    let Some(raw) = ctx.kv().get(key) else { return Ok(0); };

    match StringEncoding::decode(raw) {
        StringEncoding::Int(v) => Ok(v),
        StringEncoding::Raw(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            text.parse::<i64>().map_err(|_| invalid_input("value is not an integer"))
        }
        StringEncoding::Float(_) => Err(invalid_input("value is not an integer")),
    }
}

crate::submit_command!(IncrCommand);
