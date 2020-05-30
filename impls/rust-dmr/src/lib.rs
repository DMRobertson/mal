#[macro_use]
extern crate lazy_static;

pub mod cmdline;
pub mod environment;
pub mod evaluator;
pub mod interpreter;
pub mod printer;
pub mod reader;

mod strings;
mod tokens;
mod types;

pub use types::MalObject;
