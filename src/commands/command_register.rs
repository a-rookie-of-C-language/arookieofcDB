use crate::commands::base_args::*;

pub trait CommandRegister {
    fn register(&self);
    fn register_args(&self, args: BaseArgs);
}