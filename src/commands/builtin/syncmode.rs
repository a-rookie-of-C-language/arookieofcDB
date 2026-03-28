use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};
use crate::storage::SyncPolicy;

use super::utils::{first_arg, invalid_input};

#[derive(Default)]
struct SyncModeCommand;

impl Command for SyncModeCommand {
    fn name(&self) -> &'static str {
        "syncmode"
    }

    fn usage(&self) -> &'static str {
        "syncmode <always|manual>"
    }

    fn description(&self) -> &'static str {
        "set wal sync policy"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mode = first_arg(args).ok_or_else(|| invalid_input("missing mode: always|manual"))?;

        match mode.to_ascii_lowercase().as_str() {
            "always" => ctx.store.set_sync_policy(SyncPolicy::Always),
            "manual" => ctx.store.set_sync_policy(SyncPolicy::Manual),
            _ => return Err(invalid_input("invalid mode: always|manual")),
        }

        Ok(CommandOutput::message("ok"))
    }
}

crate::submit_command!(SyncModeCommand);
