use crate::types::{MalSymbol, PrimitiveBinaryOp};
use std::collections::HashMap;

pub type Environment = HashMap<MalSymbol, PrimitiveBinaryOp>;

pub struct EnvironmentStack {
    envs: Vec<Environment>,
}

impl EnvironmentStack {
    fn new() -> Self {
        Self { envs: Vec::new() }
    }

    fn push_env(&mut self) {
        self.envs.push(Environment::new());
    }

    fn set(&mut self, key: &MalSymbol, value: PrimitiveBinaryOp) -> Option<PrimitiveBinaryOp> {
        let map = self
            .envs
            .iter_mut()
            .last()
            .unwrap_or_else(|| panic!("No environments in stack"));
        map.insert(key.clone(), value)
    }

    pub fn get(&self, key: &MalSymbol) -> Option<&PrimitiveBinaryOp> {
        self.envs.iter().rev().find_map(|env| env.get(key))
    }
}

impl Default for EnvironmentStack {
    fn default() -> Self {
        let mut stack = Self::new();
        stack.push_env();
        stack.set(&MalSymbol::from("+"), |x, y| x.wrapping_add(y));
        stack.set(&MalSymbol::from("-"), |x, y| x.wrapping_sub(y));
        stack.set(&MalSymbol::from("*"), |x, y| x.wrapping_mul(y));
        stack.set(&MalSymbol::from("/"), |x, y| x.wrapping_div(y)); // TODO handle div by zero
        stack
    }
}
