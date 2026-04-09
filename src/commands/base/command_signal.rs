#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandSignal {
    Continue,
    Exit,
    SwitchEngine(String),
}
