use super::command_signal::CommandSignal;

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
