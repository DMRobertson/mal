use crate::types::{HashKey, MalMap, MalObject};
use crate::{interpreter, reader, strings};
use itertools::Itertools;

pub enum Outcome {
    String(String),
    Empty,
}
pub type Result = std::result::Result<Outcome, String>;

pub fn print(result: &interpreter::Result) -> Result {
    use interpreter::Error::*;
    use reader::Error::*;
    log::debug!("print {:?}", result);
    match result {
        Ok(obj) => Ok(Outcome::String(pr_str(&obj, PrintMode::Representation))),
        Err(Read(ReadComment)) => Ok(Outcome::Empty),
        Err(Read(e)) => Err(format!("{}", e)),
        Err(Eval(e)) => Err(format!("{}", e)),
    }
}

#[derive(Clone, Copy)]
pub enum PrintMode {
    Informal,
    Representation,
}

// More idiomatic to impl Display for MalObject?
fn pr_str(object: &MalObject, mode: PrintMode) -> String {
    match object {
        // TODO not sure that passing the mode through here is the right choice.
        // Think we ought to distinguish ala Python between str() and repr().
        MalObject::List(elements) => {
            format!("({})", elements.iter().map(|x| pr_str(x, mode)).join(" "))
        }
        MalObject::Vector(elements) => {
            format!("[{}]", elements.iter().map(|x| pr_str(x, mode)).join(" "))
        }
        MalObject::Map(map) => format!("{{{}}}", print_map_contents(map)),
        MalObject::Integer(value) => value.to_string(),
        MalObject::Symbol(s) => s.name.clone(),
        MalObject::Nil => String::from("nil"),
        MalObject::String(payload) => print_as_string(payload, mode),
        MalObject::Keyword(payload) => print_as_keyword(payload),
        MalObject::Bool(payload) => String::from(if *payload { "true" } else { "false" }),
        MalObject::PrimitiveBinaryOp(_) => "PRIMITIVE_BINARY_OP".to_string(),
    }
}

fn print_as_string(payload: &str, mode: PrintMode) -> String {
    match mode {
        PrintMode::Informal => payload.to_string(),
        PrintMode::Representation => strings::print_string_repr(payload),
    }
}

fn print_as_keyword(payload: &str) -> String {
    format!(":{}", payload)
}

fn print_map_contents(map: &MalMap) -> String {
    let mut output = String::new();
    for (key, value) in map.iter() {
        output.push_str(&match key {
            HashKey::String(s) => print_as_string(&s, PrintMode::Representation),
            HashKey::Keyword(s) => print_as_keyword(&s),
        });
        output.push(' ');
        output.push_str(&pr_str(&value, PrintMode::Representation));
        output.push(' ');
    }
    // Remove last space
    output.pop();
    output
}
