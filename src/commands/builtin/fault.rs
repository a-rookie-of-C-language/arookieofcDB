use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};
use crate::key_codec::KeyEncoding;
use crate::storage::FaultTarget;
use crate::value_codec::StringEncoding;

use super::utils::invalid_input;

#[derive(Default)]
struct FaultCommand;

impl Command for FaultCommand {
    fn name(&self) -> &'static str {
        "fault"
    }

    fn usage(&self) -> &'static str {
        "fault <cache-only|disk-only> <key> <value>"
    }

    fn description(&self) -> &'static str {
        "inject inconsistency in hybrid engine for demo/testing"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mut parts = args.split_whitespace();
        let target_raw = parts
            .next()
            .ok_or_else(|| invalid_input("missing target, use cache-only|disk-only"))?;
        let key_raw = parts
            .next()
            .ok_or_else(|| invalid_input("missing key for fault"))?;
        let value_raw = parts
            .next()
            .ok_or_else(|| invalid_input("missing value for fault"))?;

        if parts.next().is_some() {
            return Err(invalid_input("usage: fault <cache-only|disk-only> <key> <value>"));
        }

        let target = match target_raw.to_ascii_lowercase().as_str() {
            "cache-only" => FaultTarget::CacheOnly,
            "disk-only" => FaultTarget::DiskOnly,
            _ => return Err(invalid_input("invalid target, use cache-only|disk-only")),
        };

        let key = KeyEncoding::from_input(key_raw);
        let value = StringEncoding::from_input(value_raw).encode();

        ctx.store.inject_fault(target, key, value)?;

        let target_name = match target {
            FaultTarget::CacheOnly => "cache-only",
            FaultTarget::DiskOnly => "disk-only",
        };

        Ok(CommandOutput::message(format!(
            "fault injected target={} key={}",
            target_name, key_raw
        )))
    }
}

crate::submit_command!(FaultCommand);
