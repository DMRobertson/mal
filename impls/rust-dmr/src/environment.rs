use crate::core;
use crate::types::{MalObject, MalSymbol};
use std::collections::HashMap;

pub struct Environment<'a> {
    data: HashMap<MalSymbol, MalObject>,
    parent: Option<&'a Environment<'a>>,
}

impl Environment<'_> {
    pub fn set<T>(&mut self, key: T, value: MalObject) -> Option<MalObject>
    where
        T: Into<MalSymbol>,
    {
        self.data.insert(key.into(), value)
    }

    // The guide would have us call this "find", but it seems more rustic for get to return an Option.
    pub fn get(&self, key: &MalSymbol) -> Option<&MalObject> {
        self.data
            .get(key)
            .or_else(|| self.parent.map(|parent| parent.get(key)).flatten())
    }

    pub(crate) fn default() -> Self {
        let mut data = HashMap::new();
        for (&name, &func) in core::CORE.iter() {
            data.insert(
                MalSymbol {
                    name: name.to_string(),
                },
                MalObject::Primitive(func.clone()),
            );
        }
        Self { data, parent: None }
    }

    pub(crate) fn spawn(&self) -> Environment {
        Environment {
            data: HashMap::new(),
            parent: Some(self),
        }
    }
}
