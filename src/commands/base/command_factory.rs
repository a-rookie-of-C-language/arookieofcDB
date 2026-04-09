use super::command::Command;

pub struct CommandFactory {
    pub create: fn() -> Box<dyn Command>,
}

inventory::collect!(CommandFactory);
