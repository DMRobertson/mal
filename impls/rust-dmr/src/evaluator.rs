use crate::environment::Environment;
use crate::types::{MalList, MalObject, MalSymbol};
use std::fmt;

pub type Result = std::result::Result<MalObject, Error>;
#[derive(Debug)]
pub enum Error {
    UnknownSymbol,
    ApplyNonList,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Evaluation error: ")?;
        match self {
            Error::UnknownSymbol => write!(f, "symbol not in environment"),
            Error::ApplyNonList => write!(f, "can only interpret lists as functions"),
        }
    }
}

pub fn eval(ast: &MalObject, env: &mut Environment) -> Result {
    use MalObject::List;
    match ast {
        List(list) if list.is_empty() => Ok(List(MalList::new())),
        List(_) => match evaluate_ast(ast, env)? {
            List(list) => apply(&list),
            _ => Err(Error::ApplyNonList),
        },
        obj => evaluate_ast(obj, env),
    }
}

fn apply(argv: &MalList) -> Result {
    use MalObject::{Integer, PrimitiveBinaryOp};
    let (head, tail) = (&argv[0], &argv[1..]);
    match (head, tail) {
        (PrimitiveBinaryOp(op), [Integer(x), Integer(y)]) => Ok(Integer(op(*x, *y))),
        _ => unimplemented!(),
    }
}

fn evaluate_ast(ast: &MalObject, env: &mut Environment) -> Result {
    match ast {
        MalObject::Symbol(s) => fetch_symbol(s, env),
        MalObject::List(list) => evaluate_list_elements(list, env),
        _ => Ok(ast.clone()),
    }
}

fn evaluate_list_elements(list: &MalList, env: &mut Environment) -> Result {
    let eval = |obj: &MalObject| eval(obj, env);
    let mapped: std::result::Result<Vec<MalObject>, Error> = list.iter().map(eval).collect();
    let objects = mapped?;
    Ok(MalObject::List(objects))
}

fn fetch_symbol(s: &MalSymbol, env: &Environment) -> Result {
    env.get(s)
        .map(|f| MalObject::PrimitiveBinaryOp(*f))
        .ok_or(Error::UnknownSymbol)
}
