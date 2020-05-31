use crate::environment::EnvironmentStack;
use crate::types::{MalList, MalMap, MalObject, MalSymbol, MalVector};
use std::fmt;

pub type Result<T = MalObject> = std::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    UnknownSymbol(String),
    ListHeadNotSymbol,
    DefError(DefError),
}

#[derive(Debug)]
pub enum DefError {
    MissingKey,
    MissingValue,
    TooManyArgs(usize),
    KeyNotASymbol,
    ValueEvaluationFailed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Evaluation error: ")?;
        match self {
            Error::UnknownSymbol(s) => write!(f, "symbol {} not found", s),
            Error::ListHeadNotSymbol => {
                write!(f, "cannot apply list whose first entry is not a symbol")
            }
            Error::DefError(e) => write!(f, "def!: {:?}", e),
        }
    }
}

pub fn eval(ast: &MalObject, env: &mut EnvironmentStack) -> Result {
    use MalObject::List;
    log::debug!("eval {:?}", ast);
    let result = match ast {
        List(list) => match list.len() {
            0 => Ok(List(MalList::new())),
            _ => apply(list, env),
        },
        _ => evaluate_ast(ast, env),
    };
    log::debug!("eval produced {:?}", result);
    result
}

fn apply(argv: &[MalObject], env: &mut EnvironmentStack) -> Result {
    use MalObject::{Integer, PrimitiveBinaryOp, Symbol};
    log::debug!("apply {:?}", argv);
    if let Symbol(MalSymbol { name }) = &argv[0] {
        match name.as_str() {
            "def!" => return apply_def(&argv[1..], env).map_err(Error::DefError),
            _ => (),
        };
    };
    let evaluated = evaluate_sequence_elementwise(argv, env)?;
    match &evaluated[0] {
        PrimitiveBinaryOp(op) => match evaluated[1..] {
            [Integer(x), Integer(y)] => Ok(Integer(op(x, y))),
            _ => panic!("apply: bad PrimitiveBinaryOp"),
        },
        _ => panic!("apply: bad MalObject {:?}", evaluated),
    }
}

fn apply_def(
    args: &[MalObject],
    env: &mut EnvironmentStack,
) -> std::result::Result<MalObject, DefError> {
    let (key, value) = match args.len() {
        0 => Err(DefError::MissingKey),
        1 => Err(DefError::MissingValue),
        2 => Ok((&args[0], &args[1])),
        n => Err(DefError::TooManyArgs(n)),
    }?;
    let key = match key {
        MalObject::Symbol(s) => Ok(s),
        _ => Err(DefError::KeyNotASymbol),
    }?;
    let value = eval(value, env).map_err(|_| DefError::ValueEvaluationFailed)?;
    env.set(key.clone(), value.clone());
    Ok(value)
}

fn evaluate_ast(ast: &MalObject, env: &mut EnvironmentStack) -> Result {
    log::debug!("eval_ast {:?}", ast);
    match ast {
        MalObject::Symbol(s) => fetch_symbol(s, env).map(|obj| obj.clone()),
        MalObject::List(list) => evaluate_list_elements(list, env),
        MalObject::Vector(vec) => evaluate_vector_elements(vec, env),
        MalObject::Map(map) => evaluate_map(map, env),
        _ => Ok(ast.clone()),
    }
}

// TODO make one generic fn tkaing MalObject::List or MalObject::Vector as a parameter?
// Are rust's enum discriminants things you can be generic over?
fn evaluate_list_elements(list: &MalList, env: &mut EnvironmentStack) -> Result {
    evaluate_sequence_elementwise(list, env).map(MalObject::List)
}

fn evaluate_vector_elements(vec: &MalVector, env: &mut EnvironmentStack) -> Result {
    evaluate_sequence_elementwise(vec, env).map(MalObject::Vector)
}

fn evaluate_map(map: &MalMap, env: &mut EnvironmentStack) -> Result {
    let mut evaluated = MalMap::new();
    for key in map.keys() {
        let old_value = map.get(key).unwrap();
        let new_value = eval(old_value, env)?;
        evaluated.insert(key.clone(), new_value);
    }
    Ok(MalObject::Map(evaluated))
}

fn evaluate_sequence_elementwise(
    seq: &[MalObject],
    env: &mut EnvironmentStack,
) -> std::result::Result<Vec<MalObject>, Error> {
    let eval = |obj: &MalObject| eval(obj, env);
    let mapped: std::result::Result<Vec<MalObject>, Error> = seq.iter().map(eval).collect();
    mapped
}

fn fetch_symbol<'a>(s: &MalSymbol, env: &'a EnvironmentStack) -> Result<&'a MalObject> {
    env.get(s).ok_or(Error::UnknownSymbol(s.name.clone()))
}
