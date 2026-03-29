use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

use super::utils::{first_arg, parse_key};

#[derive(Default)]
struct GetCommand;

impl Command for GetCommand {
    fn name(&self) -> &'static str { "get" }
    fn usage(&self) -> &'static str { "get <key>" }
    fn description(&self) -> &'static str { "get value by key" }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let key = parse_key(first_arg(args), "missing key for get")?;
        let value = ctx
            .store
            .get(&key)
            .map(|v| crate::value_codec::StringEncoding::decode(v).to_display_string())
            .unwrap_or_else(|| String::from("(nil)"));
        Ok(CommandOutput::message(value))
    }
}

crate::submit_command!(GetCommand);
