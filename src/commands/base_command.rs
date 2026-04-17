use super::base_args::ParsedArgs;

#[derive(Debug)]
pub enum CommandError {
    ParseError(String),
    ExecuteError(String),
    InvalidArgs(String),
    NotFound(String),
    InternalError(String),
}

pub type CommandResult<T> = Result<T, CommandError>;

pub struct CommandExecutor<'a, C: BaseCommand + ?Sized> {
    command: &'a C,
    args: ParsedArgs,
}

impl<'a, C: BaseCommand + ?Sized> CommandExecutor<'a, C> {
    pub fn new(command: &'a C, args: ParsedArgs) -> Self {
        Self { command, args }
    }

    pub fn execute(self) -> CommandResult<String> {
        self.command.do_execute(&self.args)
    }
}

pub trait BaseCommand: Send + Sync {
    fn name(&self) -> &str;

    fn description(&self) -> &str;

    fn parse(&self, args: Vec<String>) -> CommandResult<CommandExecutor<'_, Self>> {
        let parsed = self.do_parse(args)?;
        Ok(CommandExecutor::new(self, parsed))
    }

    fn do_parse(&self, args: Vec<String>) -> CommandResult<ParsedArgs>;

    fn do_execute(&self, args: &ParsedArgs) -> CommandResult<String>;
}
