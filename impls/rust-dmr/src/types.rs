use std::collections::HashMap;

pub type MalList = Vec<MalObject>;
pub type MalVector = Vec<MalObject>;
pub type MalMap = HashMap<HashKey, MalObject>;

#[derive(Debug)]
pub enum MalObject {
    List(MalList),
    Vector(MalVector),
    Map(MalMap),
    Integer(i64),
    Symbol(String),
    String(String),
    Keyword(String),
    Bool(bool),
    Nil,
}

#[derive(Debug, PartialEq, Eq, Hash)]
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

pub(crate) fn build_map(entries: MalVector) -> Result<MalMap, MapError> {
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
    Ok(map)
}
