use crate::commands::base_command::*;
use crate::commands::command_register::*;
use crate::commands::base_args::*;
use macros::component;

#[derive(Default)]
#[component]
pub struct SetCommand;

#[derive(Default)]
struct SetArgs {
    base_args: Vec<BaseArgs>,
}

impl SetArgs {
    pub fn add_arg(&mut self, base_args: BaseArgs) {
        self.base_args.push(base_args);
    }
}

impl BaseCommand for SetCommand {
    fn name(&self) -> &str {
        "set"
    }

    fn description(&self) -> &str {
        "set a value"
    }

    fn parse(&self, args: Vec<BaseArgs>) {
        let _args = args;
    }

    fn execute(&self) {
    }
}

impl CommandRegister for SetCommand {
    fn register(&self) {
        println!("register set command");
    }

    fn register_args(&self, args: BaseArgs) {
        let mut set_args = SetArgs::default();
        set_args.add_arg(args);
    }
}
