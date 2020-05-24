use crate::types::{HashKey, MalMap, MalObject};
use itertools::Itertools;

// More idiomatic to impl Display for MalObject?
pub fn pr_str(object: &MalObject) -> String {
    // TODO should this really produce owned Strings? Allocations galore?
    // Meh. Toy project. Make it work first and learn from it.
    match object {
        MalObject::List(elements) => format!("({})", elements.iter().map(pr_str).join(" ")),
        MalObject::Vector(elements) => format!("[{}]", elements.iter().map(pr_str).join(" ")),
        MalObject::Map(map) => format!("{{{}}}", print_map_contents(map)),
        MalObject::Integer(value) => value.to_string(),
        MalObject::Symbol(name) => name.clone(),
        MalObject::Nil => String::from("nil"),
        MalObject::String(payload) => print_as_string(payload),
        lhs => unimplemented!("{:?}", lhs),
    }
}

fn print_as_string(payload: &str) -> String {
    // TODO escape double quotes and backslashes
    format!("\"{}\"", payload)
}

fn print_as_keyword(payload: &str) -> String {
    format!(":{}", payload)
}

fn print_map_contents(map: &MalMap) -> String {
    let mut output = String::new();
    for (key, value) in map.iter() {
        output.push_str(&match key {
            HashKey::String(s) => print_as_string(&s),
            HashKey::Keyword(s) => print_as_keyword(&s),
        });
        output.push(' ');
        output.push_str(&pr_str(&value));
        output.push(' ');
    }
    // Remove last space
    output.pop();
    output
}
