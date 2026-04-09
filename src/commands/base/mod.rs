pub mod command_signal;
pub mod command_output;
pub mod status_snapshot;
pub mod command_context;
pub mod command;
pub mod command_factory;
pub mod registrable_command;
pub mod command_registry;

pub use command_signal::*;
pub use command_output::*;
pub use status_snapshot::*;
pub use command_context::*;
pub use command::*;
pub use command_factory::*;
pub use registrable_command::*;
pub use command_registry::*;

#[macro_export]
macro_rules! submit_command {
    ($command_ty:ty) => {
        inventory::submit! {
            $crate::commands::base::CommandFactory {
                create: <$command_ty as $crate::commands::base::RegistrableCommand>::create_boxed
            }
        }
    };
}
