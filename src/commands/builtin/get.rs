use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

use super::utils::{first_arg, parse_i64};

#[derive(Default)]
struct GetCommand;

impl Command for GetCommand {
    fn name(&self) -> &'static str {
        "get"
    }

    fn usage(&self) -> &'static str {
        "get <key>"
    }

    fn description(&self) -> &'static str {
        "get value by key"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let key = parse_i64(first_arg(args), "missing key for get")?;
        let value = ctx
            .store
            .get(key)
            .map(|v| String::from_utf8_lossy(v).to_string())
            .unwrap_or_else(|| String::from("(nil)"));
        Ok(CommandOutput::message(value))
    }
}

crate::submit_command!(GetCommand);
