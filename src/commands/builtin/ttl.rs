use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};
use crate::storage::TtlState;

use super::utils::parse_key;

#[derive(Default)]
struct TtlCommand;

impl Command for TtlCommand {
    fn name(&self) -> &'static str { "ttl" }
    fn usage(&self) -> &'static str { "ttl <key>" }
    fn description(&self) -> &'static str { "get key time-to-live in seconds" }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mut parts = args.split_whitespace();
        let key = parse_key(parts.next(), "missing key for ttl")?;

        let out = match ctx.store.ttl(&key)? {
            TtlState::NotFound => -2,
            TtlState::NoExpire => -1,
            TtlState::Seconds(sec) => sec,
        };

        Ok(CommandOutput::message(out.to_string()))
    }
}

crate::submit_command!(TtlCommand);
