use crate::environment::Environment;
use crate::strings::BuildError;
use crate::tokens::StringLiteral;
use crate::{evaluator, strings};
use itertools::Itertools;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::ops::{RangeFrom, RangeInclusive};
use std::rc::Rc;

pub type MalList = Vec<MalObject>;
pub type MalVector = Vec<MalObject>;
pub type MalMap = HashMap<HashKey, MalObject>;
pub type MalInt = isize;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct MalSymbol {
    pub name: String,
}

impl<T> From<T> for MalSymbol
where
    T: Into<String>,
{
    fn from(item: T) -> Self {
        Self { name: item.into() }
    }
}

#[derive(Debug, Clone)]
pub enum Arity {
    Between(RangeInclusive<usize>),
    AtLeast(RangeFrom<usize>),
}

impl Arity {
    pub(crate) const fn exactly(n: usize) -> Self {
        Self::Between(n..=n)
    }

    pub(crate) const fn at_least(n: usize) -> Self {
        Self::AtLeast(n..)
    }

    pub(crate) fn contains(&self, n: usize) -> bool {
        match self {
            Self::Between(range) => range.contains(&n),
            Self::AtLeast(range) => range.contains(&n),
        }
    }
}

impl fmt::Display for Arity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arity::Between(r) => {
                if r.start() == r.end() {
                    write!(f, "exactly {}", r.start())
                } else {
                    write!(f, "from {} to {}", r.start(), r.end())
                }
            }
            Arity::AtLeast(r) => write!(f, "At least {}", r.start),
        }
    }
}

pub struct PrimitiveFn {
    pub name: &'static str,
    pub arity: Arity,
    pub fn_ptr: fn(&[MalObject]) -> evaluator::Result,
}

impl fmt::Debug for PrimitiveFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "primitive function #<{}>", self.name)
    }
}


#[derive(Debug, Clone)]
pub enum MalObject {
    Nil,
    Integer(MalInt),
    Bool(bool),
    String(String),
    Symbol(MalSymbol),
    Keyword(String),
    List(Rc<MalList>),
    Vector(Rc<MalVector>),
    Map(Rc<MalMap>),
    Primitive(&'static PrimitiveFn),
}

pub(crate) fn truthy(obj: &MalObject) -> bool {
    use MalObject::*;
    match obj {
        List(_) | Vector(_) | Map(_) | Integer(_) | Symbol(_) | String(_) | Keyword(_)
        | Primitive(_) => true,
        Bool(t) => *t,
        Nil => false,
    }
}

#[derive(Debug)]
pub enum TypeMismatch {
    NotAnInt,
    NotAList,
}

impl TryFrom<&MalObject> for MalInt {
    type Error = TypeMismatch;

    fn try_from(value: &MalObject) -> Result<Self, Self::Error> {
        match value {
            MalObject::Integer(x) => Ok(*x),
            _ => Err(TypeMismatch::NotAnInt),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
// TODO Copying the string here. Is there a better way?
pub enum HashKey {
    String(String),
    Keyword(String),
}

#[derive(Debug)]
pub enum MapError {
    MissingValue,
    UnhashableKey, // TODO include the key that wasn't hashable, or at least its position
}

pub(crate) fn build_map(entries: MalVector) -> Result<MalObject, MapError> {
    if entries.len() % 2 == 1 {
        return Err(MapError::MissingValue);
    }
    let mut map = MalMap::new();
    for (key, value) in entries.into_iter().tuples() {
        let key = match key {
            MalObject::String(s) => Ok(HashKey::String(s)),
            MalObject::Keyword(s) => Ok(HashKey::Keyword(s)),
            _ => Err(MapError::UnhashableKey),
        }?;
        map.insert(key, value);
        // TODO detect duplicate keys?
    }
    Ok(MalObject::Map(Rc::new(map)))
}

pub(crate) fn build_symbol(chars: &str) -> MalObject {
    MalObject::Symbol(MalSymbol::from(chars))
}

pub(crate) fn build_keyword(chars: &str) -> MalObject {
    MalObject::Keyword(String::from(chars))
}

pub(crate) fn build_string(src: &StringLiteral) -> Result<MalObject, BuildError> {
    strings::build_string(src.payload).map(MalObject::String)
}

impl MalObject {
    pub(crate) fn new_sequence() -> Rc<Vec<Self>> {
        Rc::new(Vec::new())
    }
    pub(crate) fn new_list() -> Self {
        Self::List(Self::new_sequence())
    }
    pub(crate) fn wrap_list(elements: Vec<MalObject>) -> Self {
        Self::List(Rc::new(elements))
    }
    pub(crate) fn wrap_vector(elements: Vec<MalObject>) -> Self {
        Self::Vector(Rc::new(elements))
    }
}
