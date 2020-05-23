use crate::types::MalObject;
use itertools::Itertools;

// More idiomatic to impl Display for MalObject?
pub fn pr_str(object: &MalObject) -> String {
    match object {
        MalObject::List(elements) => format!("({})", elements.iter().map(pr_str).join(" ")),
        MalObject::Integer(value) => value.to_string(),
        MalObject::Symbol(name) => name.clone(),
        MalObject::Nil => String::from("nil"),
        lhs => panic!("Unimplemented {:#?}", lhs),
    }
}
