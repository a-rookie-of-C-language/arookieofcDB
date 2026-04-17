use crate::commands::base_args::{ArgsType, BaseArgs, ParsedArgs};
use crate::commands::base_command::{BaseCommand, CommandError, CommandResult};
use crate::ioc::application::Application;
use crate::storage::db_engine::DbEngine;
use macros::component;

#[derive(Default)]
#[component]
pub struct GetCommand;

impl BaseCommand for GetCommand {
    fn name(&self) -> &str {
        "get"
    }

    fn description(&self) -> &str {
        "Get the value of a key"
    }

    fn do_parse(&self, args: Vec<String>) -> CommandResult<ParsedArgs> {
        if args.len() < 1 {
            return Err(CommandError::InvalidArgs(
                "wrong number of arguments for 'get' command".to_string(),
            ));
        }

        let key = args[0].clone();

        let base_args = vec![BaseArgs {
            arg_name: "key".to_string(),
            arg_value: ArgsType::String(key),
            args: crate::commands::base_args::Args::Long,
            description: "key to get".to_string(),
            help: "The key to retrieve".to_string(),
        }];

        Ok(ParsedArgs::from_vec(base_args))
    }

    fn do_execute(&self, args: &ParsedArgs) -> CommandResult<String> {
        let key = args
            .get_string("key")
            .ok_or_else(|| CommandError::InvalidArgs("missing 'key' argument".to_string()))?;

        let db_engine = Application::get_bean("DbEngine")
            .ok_or_else(|| CommandError::InternalError("DbEngine not found".to_string()))?;
        
        let engine = db_engine.downcast_ref::<DbEngine>()
            .ok_or_else(|| CommandError::InternalError("Failed to cast to DbEngine".to_string()))?;

        match engine.get(key.as_bytes()) {
            Some(value) => String::from_utf8(value)
                .map_err(|_| CommandError::InternalError("value is not valid UTF-8".to_string())),
            None => Ok("(nil)".to_string()),
        }
    }
}
