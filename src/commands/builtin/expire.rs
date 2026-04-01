use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

use super::utils::{parse_i64, parse_key};

#[derive(Default)]
struct ExpireCommand;

impl Command for ExpireCommand {
    fn name(&self) -> &'static str { "expire" }
    fn usage(&self) -> &'static str { "expire <key> <seconds>" }
    fn description(&self) -> &'static str { "set key expiration in seconds" }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mut parts = args.split_whitespace();
        let key = parse_key(parts.next(), "missing key for expire")?;
        let seconds = parse_i64(parts.next(), "missing seconds for expire")?;

        if seconds <= 0 {
            let removed = ctx.kv().delete(&key)?.is_some();
            return Ok(CommandOutput::message(if removed { "1" } else { "0" }));
        }

        let changed = ctx.ttl().expire(&key, seconds as u64)?;
        Ok(CommandOutput::message(if changed { "1" } else { "0" }))
    }
}

crate::submit_command!(ExpireCommand);
