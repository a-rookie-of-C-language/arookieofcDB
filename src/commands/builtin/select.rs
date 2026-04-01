use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

use super::utils::{invalid_input, parse_key};

#[derive(Default)]
struct SelectCommand;

impl Command for SelectCommand {
    fn name(&self) -> &'static str { "select" }
    fn usage(&self) -> &'static str { "select [--disk] <key>" }
    fn description(&self) -> &'static str { "select value by key (sql-like select)" }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mut parts = args.split_whitespace();
        let first = parts.next();

        let (force_disk, key_raw) = match first {
            Some("--disk") => (true, parts.next()),
            Some(other) => (false, Some(other)),
            None => return Err(invalid_input("missing key for select")),
        };

        let key = parse_key(key_raw, "missing key for select")?;

        let out = if force_disk {
            ctx.disk_read()
                .get_disk_only(&key)
                .map(|v| crate::value_codec::StringEncoding::decode(&v).to_display_string())
        } else {
            ctx.kv()
                .get(&key)
                .map(|v| crate::value_codec::StringEncoding::decode(v).to_display_string())
        }
        .unwrap_or_else(|| String::from("(nil)"));

        Ok(CommandOutput::message(out))
    }
}

crate::submit_command!(SelectCommand);
