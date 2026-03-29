use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

use super::utils::{first_arg, invalid_input};

#[derive(Default)]
struct CachePolicyCommand;

impl Command for CachePolicyCommand {
    fn name(&self) -> &'static str {
        "cachepolicy"
    }

    fn usage(&self) -> &'static str {
        "cachepolicy [lru|none]"
    }

    fn description(&self) -> &'static str {
        "set/show hybrid cache policy"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let raw = first_arg(args);

        if raw.is_none() {
            let out = ctx
                .store
                .cache_policy()
                .map(|v| v.to_string())
                .unwrap_or_else(|| String::from("unsupported"));
            return Ok(CommandOutput::message(out));
        }

        let policy = raw.unwrap_or_default();
        if !matches!(policy.to_ascii_lowercase().as_str(), "lru" | "none") {
            return Err(invalid_input("invalid cache policy: use lru|none"));
        }

        ctx.store.set_cache_policy(policy)?;
        Ok(CommandOutput::message(format!("ok (cache_policy={})", policy.to_ascii_lowercase())))
    }
}

crate::submit_command!(CachePolicyCommand);
