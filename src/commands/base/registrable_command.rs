use super::command::Command;

pub trait RegistrableCommand: Command + Default + 'static {
    fn create_boxed() -> Box<dyn Command> {
        Box::new(Self::default())
    }
}

impl<T> RegistrableCommand for T where T: Command + Default + 'static {}
