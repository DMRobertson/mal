use crate::environment::Environment;
use crate::types::{MalList, MalObject, MalSymbol, MalVector};
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
        MalObject::Vector(vec) => evaluate_vector_elements(vec, env),
        _ => Ok(ast.clone()),
    }
}

// TODO make one generic fn tkaing MalObject::List or MalObject::Vector as a parameter?
// Are rust's enum discriminants things you can be generic over?
fn evaluate_list_elements(list: &MalList, env: &mut Environment) -> Result {
    evaluate_sequence_elementwise(list, env).map(MalObject::List)
}

fn evaluate_vector_elements(vec: &MalVector, env: &mut Environment) -> Result {
    evaluate_sequence_elementwise(vec, env).map(MalObject::Vector)
}

fn evaluate_sequence_elementwise(
    seq: &Vec<MalObject>,
    env: &mut Environment,
) -> std::result::Result<Vec<MalObject>, Error> {
    let eval = |obj: &MalObject| eval(obj, env);
    let mapped: std::result::Result<Vec<MalObject>, Error> = seq.iter().map(eval).collect();
    mapped
}

fn fetch_symbol(s: &MalSymbol, env: &Environment) -> Result {
    env.get(s)
        .map(|f| MalObject::PrimitiveBinaryOp(*f))
        .ok_or(Error::UnknownSymbol)
}
