use crate::types::{MalObject, MalSymbol, PrimitiveEval, PrimitiveFnRef};
use crate::{core, interpreter, prelude};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

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

#[derive(Debug)]
pub struct UnknownSymbol(pub MalSymbol);

impl Environment {
    pub fn set<T>(&self, key: T, value: MalObject) -> Option<MalObject>
    where
        T: Into<MalSymbol>,
    {
        self.data.borrow_mut().insert(key.into(), value)
    }

    // The guide would have us call this "find", and have a "get" which errors if
    // there's no value matching `key`. But it seems more rustic for get to return
    // an Option.
    pub fn get(&self, key: &MalSymbol) -> Option<MalObject> {
        match self.data.borrow().get(key) {
            // TODO is this done correctly---we clone the value?
            Some(value) => Some(value.clone()),
            None => match &self.parent {
                // TODO: nonrecursive?
                Some(parent) => parent.get(key),
                None => None,
            },
        }
    }
    pub(crate) fn fetch(&self, key: &MalSymbol) -> Result<MalObject, UnknownSymbol> {
        self.get(key).ok_or_else(|| UnknownSymbol(key.clone()))
    }

    pub fn empty() -> Self {
        Self {
            data: RefCell::new(HashMap::new()),
            parent: None,
        }
    }

    pub fn default() -> Self {
        let mut data = HashMap::new();
        for (&name, &func) in core::CORE.iter() {
            data.insert(
                MalSymbol(name.into()),
                MalObject::Primitive(PrimitiveFnRef {
                    payload: func,
                    meta: Box::new(MalObject::Nil),
                }),
            );
        }
        data.insert(
            MalSymbol("*host-language*".into()),
            MalObject::String("rust-dmr".into()),
        );
        Self {
            data: RefCell::new(data),
            parent: None,
        }
    }

    pub(crate) fn spawn_from(parent: &Rc<Environment>) -> Rc<Environment> {
        Rc::new(Environment {
            data: RefCell::new(HashMap::new()),
            parent: Some(parent.clone()),
        })
    }

    fn fmt_internal(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        for (key, value) in self.data.borrow().iter() {
            writeln!(f, "\t{} => {}", key, value)?;
        }
        match &self.parent {
            None => Ok(()),
            Some(parent) => {
                // TODO nonrecursive?
                writeln!(f, "whose parent (depth {}) contains:", depth)?;
                parent.fmt_internal(f, depth + 1)
            }
        }
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Environment with data:")?;
        if f.alternate() {
            Ok(())
        } else {
            self.fmt_internal(f, 0)
        }
    }
}

pub fn read_prelude(env: &Rc<Environment>) -> Result<(), String> {
    let result: Result<Vec<_>, _> = prelude::PRELUDE
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| interpreter::rep(s, env))
        .collect();
    result.map(|_| ())
}

pub fn add_eval(env: &Rc<Environment>) {
    let dummy = MalObject::Eval(PrimitiveEval {
        env: Rc::downgrade(env),
    });
    env.set(MalSymbol("eval".into()), dummy);
}
