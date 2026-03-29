use crate::storage::StorageEngine;
use std::io;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandSignal {
    Continue,
    Exit,
    SwitchEngine(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub message: Option<String>,
    pub signal: CommandSignal,
}

impl CommandOutput {
    pub fn none() -> Self {
        Self {
            message: None,
            signal: CommandSignal::Continue,
        }
    }

    pub fn message(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
            signal: CommandSignal::Continue,
        }
    }

    pub fn with_signal(message: Option<String>, signal: CommandSignal) -> Self {
        Self { message, signal }
    }

    pub fn switch_engine(mode: impl Into<String>) -> Self {
        let mode = mode.into();
        Self {
            message: Some(format!("switching engine to {mode}")),
            signal: CommandSignal::SwitchEngine(mode),
        }
    }
}

pub struct CommandContext<'a> {
    pub store: &'a mut dyn StorageEngine,
}

pub trait Command {
    fn name(&self) -> &'static str;
    fn usage(&self) -> &'static str;
    fn description(&self) -> &'static str;

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput>;
}

pub struct CommandFactory {
    pub create: fn() -> Box<dyn Command>,
}

inventory::collect!(CommandFactory);

pub trait RegistrableCommand: Command + Default + 'static {
    fn create_boxed() -> Box<dyn Command> {
        Box::new(Self::default())
    }
}

impl<T> RegistrableCommand for T where T: Command + Default + 'static {}

#[macro_export]
macro_rules! submit_command {
    ($command_ty:ty) => {
        inventory::submit! {
            $crate::commands::base::CommandFactory {
                create: <$command_ty as $crate::commands::base::RegistrableCommand>::create_boxed
            }
        }
    };
}

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
