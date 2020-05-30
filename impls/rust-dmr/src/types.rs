use crate::strings;
use crate::strings::BuildError;
use crate::tokens::StringLiteral;
use std::collections::HashMap;

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
    String: From<T>,
{
    fn from(item: T) -> Self {
        Self {
            name: String::from(item),
        }
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
    PrimitiveBinaryOp(PrimitiveBinaryOp),
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
    let mut entries = entries.into_iter();
    while entries.len() > 0 {
        let key = entries.next().unwrap();
        let value = entries.next().unwrap();
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
