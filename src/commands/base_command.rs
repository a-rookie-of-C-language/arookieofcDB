use crate::commands::base_args::*;
pub trait BaseCommand {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self);
    fn parse(&self, args: Vec<BaseArgs>);
}