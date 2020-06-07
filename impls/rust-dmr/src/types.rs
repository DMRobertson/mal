use crate::strings::BuildError;
use crate::tokens::StringLiteral;
use crate::{evaluator, strings};
use itertools::Itertools;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::ops::{RangeFrom, RangeInclusive};

pub type MalList = Vec<MalObject>;
pub type MalVector = Vec<MalObject>;
pub type MalMap = HashMap<HashKey, MalObject>;
pub type MalInt = i64;

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

#[derive(Debug)]
pub enum Arity {
    Bounded(RangeInclusive<usize>),
    BoundedBelow(RangeFrom<usize>),
}

impl Arity {
    pub(crate) const fn exactly(n: usize) -> Self {
        Self::Bounded(n..=n)
    }
}

impl fmt::Display for Arity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arity::Bounded(r) => {
                if r.start() == r.end() {
                    write!(f, "exactly {}", r.start())
                } else {
                    write!(f, "from {} to {}", r.start(), r.end())
                }
            }
            Arity::BoundedBelow(r) => write!(f, "At least {}", r.start),
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

pub type PrimitiveBinaryOp = fn(MalInt, MalInt) -> MalInt;

#[derive(Debug, Clone)]
pub enum MalObject {
    List(MalList),
    Vector(MalVector),
    Map(MalMap),
    Integer(MalInt),
    Symbol(MalSymbol),
    String(String),
    Keyword(String),
    Bool(bool),
    Nil,
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
    Ok(MalObject::Map(map))
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
