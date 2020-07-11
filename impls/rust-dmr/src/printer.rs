use crate::types::{Closure, HashKey, MalObject};
use crate::{interpreter, reader, strings, types};
use std::fmt;

pub enum Outcome {
    String(String),
    Empty,
}
pub type Result = std::result::Result<Outcome, String>;

pub fn print(result: &interpreter::Result) -> Result {
    use interpreter::Error::*;
    use reader::Error::*;
    match result {
        Ok(obj) => Ok(Outcome::String(pr_str(
            &obj,
            PrintMode::ReadableRepresentation,
        ))),
        Err(Read(ReadComment)) => Ok(Outcome::Empty),
        Err(Read(e)) => Err(format!("{}", e)),
        Err(Eval(e)) => Err(format!("{}", e)),
    }
}

#[derive(Clone, Copy)]
pub enum PrintMode {
    ReadableRepresentation,
    Directly,
}

pub(crate) fn pr_str(object: &MalObject, mode: PrintMode) -> String {
    match object {
        MalObject::String(payload) => print_as_string(payload, mode),
        MalObject::List(x) => {
            let mut output: String = "(".into();
            write_sequence(&mut output, x, mode).unwrap();
            output.push(')');
            output
        }
        MalObject::Vector(x) => {
            let mut output: String = "[".into();
            write_sequence(&mut output, x, mode).unwrap();
            output.push(']');
            output
        }
        _ => format!("{}", object),
    }
}

fn print_as_string(payload: &str, mode: PrintMode) -> String {
    match mode {
        PrintMode::ReadableRepresentation => strings::string_repr(payload),
        PrintMode::Directly => payload.to_string(),
    }
}

impl fmt::Display for Closure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({} ({}) {})",
            match self.is_macro {
                true => "fn*-macro",
                false => "fn*",
            },
            self.parameters,
            self.body,
        )
    }
}

impl fmt::Display for types::Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(atom {})", self.borrow_payload())
    }
}

impl fmt::Display for types::MalSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn write_sequence(f: &mut impl fmt::Write, seq: &[MalObject], mode: PrintMode) -> fmt::Result {
    let mut iter = seq.iter().peekable();
    while let Some(obj) = iter.next() {
        write!(f, "{}", pr_str(obj, mode))?;
        if let Some(_) = iter.peek() {
            write!(f, " ")?;
        }
    }
    Ok(())
}

impl fmt::Display for types::MalList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")
            .and_then(|_| write_sequence(f, self, PrintMode::ReadableRepresentation))
            .and_then(|_| write!(f, ")"))
    }
}

impl fmt::Display for types::MalVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")
            .and_then(|_| write_sequence(f, self, PrintMode::ReadableRepresentation))
            .and_then(|_| write!(f, "]"))
    }
}

impl fmt::Display for types::MalMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut iter = self.iter().peekable();
        while let Some((key, value)) = iter.next() {
            match key {
                HashKey::String(s) => write!(
                    f,
                    "{}",
                    print_as_string(&s, PrintMode::ReadableRepresentation)
                ),
                HashKey::Keyword(s) => write!(f, ":{}", s),
            }?;
            write!(f, " {}", value)?;
            if let Some(_) = iter.peek() {
                write!(f, " ")?;
            };
        }
        write!(f, "}}")?;
        Ok(())
    }
}

impl fmt::Display for MalObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use MalObject::*;
        match self {
            Nil => write!(f, "nil"),
            Integer(x) => write!(f, "{}", x),
            Bool(x) => write!(f, "{}", x),
            String(x) => write!(f, "{:?}", x),
            Symbol(x) => write!(f, "{}", x),
            Keyword(x) => write!(f, ":{}", x),
            List(x) => write!(f, "{}", x),
            Vector(x) => write!(f, "{}", x),
            Map(x) => write!(f, "{}", x),
            Primitive(x) => write!(f, "{}", x.name),
            Closure(x) => write!(f, "{}", x),
            Eval(_) => write!(f, "eval"),
            Atom(x) => write!(f, "{}", x),
        }
    }
}
