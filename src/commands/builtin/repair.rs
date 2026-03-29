use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};
use crate::storage::RepairTarget;

#[derive(Default)]
struct RepairCommand;

impl Command for RepairCommand {
    fn name(&self) -> &'static str {
        "repair"
    }

    fn usage(&self) -> &'static str {
        "repair --to <disk|cache>"
    }

    fn description(&self) -> &'static str {
        "repair cache/disk inconsistency by choosing source of truth"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mut parts = args.split_whitespace();
        let flag = parts.next();
        let target_raw = parts.next();

        if flag != Some("--to") || target_raw.is_none() || parts.next().is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "usage: repair --to <disk|cache>",
            ));
        }

        let target = match target_raw.unwrap_or_default().to_ascii_lowercase().as_str() {
            "disk" => RepairTarget::Disk,
            "cache" => RepairTarget::Cache,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "invalid repair target, use disk|cache",
                ));
            }
        };

        let report = ctx.store.repair_consistency(target)?;
        let target_name = match report.target {
            RepairTarget::Disk => "disk",
            RepairTarget::Cache => "cache",
        };

        Ok(CommandOutput::message(format!(
            "repair target={} total={} only_in_cache={} only_in_disk={} value_mismatch={}",
            target_name,
            report.total_repairs(),
            report.repaired_only_in_cache,
            report.repaired_only_in_disk,
            report.repaired_value_mismatches,
        )))
    }
}

crate::submit_command!(RepairCommand);
