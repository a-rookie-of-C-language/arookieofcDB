use std::io;
use crate::storage::StorageEngine;
use super::command::Command;
use super::command_context::CommandContext;
use super::command_output::CommandOutput;
use super::command_signal::CommandSignal;
use super::command_factory::CommandFactory;

pub struct CommandRegistry {
    commands: Vec<Box<dyn Command>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let commands = inventory::iter::<CommandFactory>
            .into_iter()
            .map(|factory| (factory.create)())
            .collect();

        Self { commands }
    }

    pub fn execute_line(
        &self,
        store: &mut dyn StorageEngine,
        line: &str,
    ) -> io::Result<CommandOutput> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(CommandOutput::none());
        }

        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or_default().to_ascii_lowercase();
        let args = parts.next().unwrap_or_default().trim_start();

        if name == "help" {
            return Ok(CommandOutput::message(self.help_text()));
        }

        if name == "exit" || name == "quit" {
            return Ok(CommandOutput::with_signal(
                Some(String::from("bye")),
                CommandSignal::Exit,
            ));
        }

        let mut ctx = CommandContext { store };
        if let Some(command) = self.find_command(&name) {
            return command.execute(&mut ctx, args);
        }

        Ok(CommandOutput::message(format!("unknown command: {name}")))
    }

    pub fn help_text(&self) -> String {
        let mut lines = vec![String::from("commands:")];

        let mut rows: Vec<_> = self
            .commands
            .iter()
            .map(|cmd| (cmd.name(), cmd.usage(), cmd.description()))
            .collect();

        rows.sort_by(|a, b| a.0.cmp(b.0));

        for (_, usage, description) in rows {
            lines.push(format!("  {usage}  - {description}"));
        }

        lines.push(String::from("  help  - show this help"));
        lines.push(String::from("  exit | quit  - exit cli"));

        lines.join("\n")
    }

    fn find_command(&self, name: &str) -> Option<&dyn Command> {
        self.commands
            .iter()
            .find(|cmd| cmd.name() == name || cmd.aliases().iter().any(|alias| *alias == name))
            .map(|cmd| cmd.as_ref())
    }
}
