#[macro_use]
extern crate lazy_static;

pub mod cmdline;
pub mod environment;
pub mod evaluator;
pub mod interpreter;
pub mod prelude;
pub mod printer;
pub mod reader;
pub mod special_forms;
pub mod types;

mod core;
mod strings;
mod tokens;
