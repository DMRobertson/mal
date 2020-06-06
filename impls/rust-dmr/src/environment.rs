use crate::types::{MalObject, MalSymbol};
use std::collections::HashMap;

pub type Environment = HashMap<MalSymbol, MalObject>;

pub struct EnvironmentStack {
    envs: Vec<Environment>,
}

impl EnvironmentStack {
    pub(crate) fn push(&mut self) {
        self.envs.push(Environment::new());
    }

    pub(crate) fn pop(&mut self) {
        self.envs.pop();
    }

    pub fn set<T>(&mut self, key: T, value: MalObject) -> Option<MalObject>
    where
        T: Into<MalSymbol>,
    {
        let map = self
            .envs
            .iter_mut()
            .last()
            .unwrap_or_else(|| panic!("No environments in stack"));
        map.insert(key.into(), value)
    }

    pub fn get(&self, key: &MalSymbol) -> Option<&MalObject> {
        self.envs.iter().rev().find_map(|env| env.get(key))
    }
}

impl Default for EnvironmentStack {
    fn default() -> Self {
        use MalObject::PrimitiveBinaryOp;
        let mut stack = Self { envs: Vec::new() };
        stack.push();
        stack.set("+", PrimitiveBinaryOp(|x, y| x.wrapping_add(y)));
        stack.set("-", PrimitiveBinaryOp(|x, y| x.wrapping_sub(y)));
        stack.set("*", PrimitiveBinaryOp(|x, y| x.wrapping_mul(y)));
        stack.set("/", PrimitiveBinaryOp(|x, y| x.wrapping_div(y))); // TODO handle div by zero
        stack
    }
}
