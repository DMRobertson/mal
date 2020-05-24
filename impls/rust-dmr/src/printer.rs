use crate::types::MalObject;
use itertools::Itertools;

// More idiomatic to impl Display for MalObject?
pub fn pr_str(object: &MalObject) -> String {
    // TODO should this really produce owned Strings? Allocations galore?
    // Meh. Toy project. Make it work first and learn from it.
    match object {
        MalObject::List(elements) => format!("({})", elements.iter().map(pr_str).join(" ")),
        MalObject::Vector(elements) => format!("[{}]", elements.iter().map(pr_str).join(" ")),
        MalObject::Integer(value) => value.to_string(),
        MalObject::Symbol(name) => name.clone(),
        MalObject::Nil => String::from("nil"),
        MalObject::String(payload) => format!("\"{}\"", payload),
        lhs => unimplemented!("{:?}", lhs),
    }
}
