use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};
use crate::storage::RepairMode;

use super::utils::first_arg;

#[derive(Default)]
struct RepairModeCommand;

impl Command for RepairModeCommand {
    fn name(&self) -> &'static str {
        "repairmode"
    }

    fn usage(&self) -> &'static str {
        "repairmode [off|read|write|always]"
    }

    fn description(&self) -> &'static str {
        "get/set automatic repair mode"
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let arg = first_arg(args);

        if arg.is_none() {
            let out = ctx
                .store
                .repair_mode()
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| String::from("none"));
            return Ok(CommandOutput::message(out));
        }

        let raw = arg.unwrap_or_default();
        let Some(mode) = RepairMode::from_input(raw) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid repair mode: use off|read|write|always",
            ));
        };

        ctx.store.set_repair_mode(mode)?;
        Ok(CommandOutput::message(format!("ok (repair_mode={})", mode.as_str())))
    }
}

crate::submit_command!(RepairModeCommand);
