use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};
use crate::storage::{ConsistencyDiffKind, ConsistencyReport};

#[derive(Default)]
struct VerifyCommand;

impl VerifyCommand {
    fn render(report: &ConsistencyReport) -> String {
        let mut lines = vec![
            format!("verify: cache_keys={}, disk_keys={}", report.cache_keys, report.disk_keys),
            format!(
                "issues: total={}, only_in_cache={}, only_in_disk={}, value_mismatch={}",
                report.total_issues(),
                report.only_in_cache,
                report.only_in_disk,
                report.value_mismatches
            ),
        ];

        if report.samples.is_empty() {
            lines.push(String::from("sample: clean"));
            return lines.join("\n");
        }

        lines.push(String::from("sample:"));
        for diff in &report.samples {
            let kind = match diff.kind {
                ConsistencyDiffKind::OnlyInCache => "only_in_cache",
                ConsistencyDiffKind::OnlyInDisk => "only_in_disk",
                ConsistencyDiffKind::ValueMismatch => "value_mismatch",
            };
            lines.push(format!("  - {} [{}]", diff.key.to_display_string(), kind));
        }

        lines.join("\n")
    }
}

impl Command for VerifyCommand {
    fn name(&self) -> &'static str {
        "verify"
    }

    fn usage(&self) -> &'static str {
        "verify"
    }

    fn description(&self) -> &'static str {
        "verify cache/disk consistency in current engine"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        if !args.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "usage: verify",
            ));
        }

        let report = ctx.store.verify_consistency()?;
        Ok(CommandOutput::message(Self::render(&report)))
    }
}

crate::submit_command!(VerifyCommand);
