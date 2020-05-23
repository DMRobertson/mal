pub type MalList = Vec<MalObject>;

#[derive(Debug)]
pub enum MalObject {
    List(MalList),
    Integer(i64),
    Symbol(String),
    String(String),
    Keyword(String),
    Bool(bool),
    Nil,
}
