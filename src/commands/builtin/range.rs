use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};
use crate::value_codec::StringEncoding;

use super::utils::parse_key;

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
        "list key-value pairs in inclusive key range"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mut parts = args.split_whitespace();
        let start = parse_key(parts.next(), "missing start key for range")?;
        let end = parse_key(parts.next(), "missing end key for range")?;

        if parts.next().is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "usage: range <start> <end>",
            ));
        }

        if start > end {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "range start must be <= end",
            ));
        }

        let rows = ctx.range_read().range(&start, &end)?;
        if rows.is_empty() {
            return Ok(CommandOutput::message("(empty)"));
        }

        let lines = rows
            .into_iter()
            .map(|(key, value)| {
                format!(
                    "{} => {}",
                    key.to_display_string(),
                    StringEncoding::decode(&value).to_display_string()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(CommandOutput::message(lines))
    }
}

crate::submit_command!(RangeCommand);
