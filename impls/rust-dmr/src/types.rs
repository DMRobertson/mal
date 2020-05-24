pub type MalList = Vec<MalObject>;
pub type MalVector = Vec<MalObject>;

#[derive(Debug)]
pub enum MalObject {
    List(MalList),
    Vector(MalVector),
    Integer(i64),
    Symbol(String),
    String(String),
    Keyword(String),
    Bool(bool),
    Nil,
}
