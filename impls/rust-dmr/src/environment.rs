use crate::core;
use crate::types::{MalObject, MalSymbol};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct Environment {
    /* Did a bit of cheating here by consulting the existing rust implementation.
     Consider the following snippet

    (def x 1)
    (fn f (+ x 1))
    (fn g (* x 2))
    ....

    When we evaluate these expressions, the call to `def` will require us to mutate the current environment's data. Let's call the current environment E.
    The closures f and g will get their own environments F and G whose parent is E.
    F and G need to defer to E for lookups, so they need read-only access.
    But Rust doesn't let us have multiple borrows and a mutable borrow. We need that mutability too,
    because for all we know the next line is `(def x 20)`.

    We are only ever going to mutate the data, not the parent. So wrap the data in a RefCell rather than the whole strcut.

    */
    data: RefCell<HashMap<MalSymbol, MalObject>>,
    parent: Option<Rc<Environment>>,
}

impl Environment {
    pub fn set<T>(&self, key: T, value: MalObject) -> Option<MalObject>
    where
        T: Into<MalSymbol>,
    {
        self.data.borrow_mut().insert(key.into(), value)
    }

    // The guide would have us call this "find", and have a "get" which errors if
    // there's no value mathching `key`. But it seems more rustic for get to return
    // an Option.
    pub fn get(&self, key: &MalSymbol) -> Option<MalObject> {
        match self.data.borrow().get(key) {
            Some(value) => Some(value.clone()),
            None => match &self.parent {
                Some(parent) => parent.get(key),
                None => None,
            },
        }
    }

    pub(crate) fn default() -> Self {
        let mut data = HashMap::new();
        for (&name, &func) in core::CORE.iter() {
            data.insert(
                MalSymbol {
                    name: name.to_string(),
                },
                MalObject::Primitive(func),
            );
        }
        Self {
            data: RefCell::new(data),
            parent: None,
        }
    }

    pub(crate) fn spawn_from(parent: &Rc<Environment>) -> Environment {
        Environment {
            data: RefCell::new(HashMap::new()),
            parent: Some(parent.clone()),
        }
    }
}
