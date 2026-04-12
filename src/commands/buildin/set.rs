use crate::commands::base_args::{ArgsType, BaseArgs, ParsedArgs};
use crate::commands::base_command::{BaseCommand, CommandError, CommandResult};
use macros::component;

#[derive(Default)]
#[component]
pub struct SetCommand;

impl BaseCommand for SetCommand {
    fn name(&self) -> &str {
        "set"
    }

    fn description(&self) -> &str {
        "Set the string value of a key"
    }

    fn do_parse(&self, args: Vec<String>) -> CommandResult<ParsedArgs> {
        if args.len() < 2 {
            return Err(CommandError::InvalidArgs(
                "wrong number of arguments for 'set' command".to_string(),
            ));
        }

        let key = args[0].clone();
        let value = args[1].clone();

        let base_args = vec![
            BaseArgs {
                arg_name: "key".to_string(),
                arg_value: ArgsType::String(key),
                args: crate::commands::base_args::Args::Long,
                description: "key to set".to_string(),
                help: "The key where the value will be stored".to_string(),
            },
            BaseArgs {
                arg_name: "value".to_string(),
                arg_value: ArgsType::String(value),
                args: crate::commands::base_args::Args::Long,
                description: "value to set".to_string(),
                help: "The value to store at the key".to_string(),
            },
        ];

        Ok(ParsedArgs::from_vec(base_args))
    }

    fn do_execute(&self, args: &ParsedArgs) -> CommandResult<String> {
        let key = args
            .get_string("key")
            .ok_or_else(|| CommandError::InvalidArgs("missing 'key' argument".to_string()))?;

        let value = args
            .get_string("value")
            .ok_or_else(|| CommandError::InvalidArgs("missing 'value' argument".to_string()))?;

        // TODO: 实际存储逻辑，通过 DAO 层操作
        println!("SET {} = {}", key, value);

        Ok("OK".to_string())
    }
}
