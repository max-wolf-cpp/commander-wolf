mod builtin_catalog;
mod builtins;
pub mod completion;
mod dispatch;
mod display;
pub mod history;
mod invocation;
mod navigation;
mod parser;
mod registry;

pub use builtins::BuiltinCommand;
pub use invocation::{CommandId, CommandInvocation};
pub use parser::{CommandLine, parse_command_line};
pub use registry::{CommandDispatch, CommandRegistry};
