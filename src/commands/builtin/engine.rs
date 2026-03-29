use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};

use super::utils::invalid_input;

#[derive(Default)]
struct EngineCommand;

impl Command for EngineCommand {
    fn name(&self) -> &'static str {
        "engine"
    }

    fn usage(&self) -> &'static str {
        "engine <memory|wal|hybrid>"
    }

    fn description(&self) -> &'static str {
        "switch runtime engine"
    }

    fn execute(&self, _ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mode = args
            .split_whitespace()
            .next()
            .ok_or_else(|| invalid_input("missing engine mode: memory|wal|hybrid"))?
            .to_ascii_lowercase();

        match mode.as_str() {
            "memory" | "wal" | "disk" | "bptree" | "hybrid" => Ok(CommandOutput::switch_engine(mode)),
            _ => Err(invalid_input("invalid engine mode: memory|wal|hybrid")),
        }
    }
}

crate::submit_command!(EngineCommand);
