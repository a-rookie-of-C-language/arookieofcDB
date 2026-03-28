use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

use super::utils::parse_i64;

#[derive(Default)]
struct RangeCommand;

impl Command for RangeCommand {
    fn name(&self) -> &'static str {
        "range"
    }

    fn usage(&self) -> &'static str {
        "range <start> <end>"
    }

    fn description(&self) -> &'static str {
        "scan key range"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mut parts = args.split_whitespace();
        let start = parse_i64(parts.next(), "missing start for range")?;
        let end = parse_i64(parts.next(), "missing end for range")?;

        let items = ctx.store.range_query(start, end);
        let output = if items.is_empty() {
            String::from("(empty)")
        } else {
            items
                .into_iter()
                .map(|(k, v)| format!("{k}={}", String::from_utf8_lossy(&v)))
                .collect::<Vec<_>>()
                .join(", ")
        };

        Ok(CommandOutput::message(output))
    }
}

crate::submit_command!(RangeCommand);
