use crate::types::{MalSymbol, PrimitiveBinaryOp};
use std::collections::HashMap;

pub type Environment = HashMap<MalSymbol, PrimitiveBinaryOp>;

pub fn repl_env() -> Environment {
    let mut env = Environment::new();
    env.insert(MalSymbol::from("+"), |x, y| x.wrapping_add(y));
    env.insert(MalSymbol::from("-"), |x, y| x.wrapping_sub(y));
    env.insert(MalSymbol::from("*"), |x, y| x.wrapping_mul(y));
    env.insert(MalSymbol::from("/"), |x, y| x.wrapping_div(y)); // TODO handle div by zero
    env
}
