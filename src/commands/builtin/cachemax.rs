use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

use super::utils::{first_arg, invalid_input};

#[derive(Default)]
struct CacheMaxCommand;

impl Command for CacheMaxCommand {
    fn name(&self) -> &'static str {
        "cachemax"
    }

    fn usage(&self) -> &'static str {
        "cachemax [max_keys]"
    }

    fn description(&self) -> &'static str {
        "set/show hybrid memory cache max keys (0 means unlimited)"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let raw = first_arg(args);

        if raw.is_none() {
            let out = ctx.cache_max_keys_string();
            return Ok(CommandOutput::message(out));
        }

        let max_keys = raw
            .unwrap_or_default()
            .parse::<usize>()
            .map_err(|_| invalid_input("invalid max_keys: must be usize"))?;

        ctx.cache_limits().set_cache_max_keys(max_keys)?;
        Ok(CommandOutput::message(format!("ok (cache_max_keys={max_keys})")))
    }
}

crate::submit_command!(CacheMaxCommand);
