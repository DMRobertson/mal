use crate::environment::EnvironmentStack;
use crate::types::{MalList, MalMap, MalObject, MalSymbol};
use itertools::Itertools;
use std::fmt;

pub type Result<T = MalObject> = std::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    UnknownSymbol(String),
    ListHeadNotSymbol,
    DefError(DefError),
    LetError(LetError),
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
            Error::LetError(e) => write!(f, "let*: {:?}", e),
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
            "let*" => return apply_let(&argv[1..], env).map_err(Error::LetError),
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

#[derive(Debug)]
pub enum DefError {
    WrongArgCount(usize),
    KeyNotASymbol,
    ValueEvaluationFailed,
}

fn apply_def(
    args: &[MalObject],
    env: &mut EnvironmentStack,
) -> std::result::Result<MalObject, DefError> {
    let (key, value) = match args.len() {
        2 => Ok((&args[0], &args[1])),
        n => Err(DefError::WrongArgCount(n)),
    }?;
    let key = match key {
        MalObject::Symbol(s) => Ok(s),
        _ => Err(DefError::KeyNotASymbol),
    }?;
    let value = eval(value, env).map_err(|_| DefError::ValueEvaluationFailed)?;
    env.set(key.clone(), value.clone());
    Ok(value)
}

#[derive(Debug)]
pub enum LetError {
    WrongArgCount(usize),
    BindingsNotSequence,
    BindingsOddLength,
    ValueEvaluationFailed,
    BindToNonSymbol,
}

fn apply_let(
    args: &[MalObject],
    env: &mut EnvironmentStack,
) -> std::result::Result<MalObject, LetError> {
    use MalObject::{List, Vector};
    let (bindings, obj) = match args.len() {
        2 => Ok((&args[0], &args[1])),
        n => Err(LetError::WrongArgCount(n)),
    }?;
    match bindings {
        List(bindings) | Vector(bindings) if bindings.len() % 2 == 0 => {
            apply_let_evaluate(bindings, obj, env)
        }
        List(_) | Vector(_) => Err(LetError::BindingsOddLength),
        _ => Err(LetError::BindingsNotSequence),
    }
}

fn apply_let_evaluate(
    bindings: &[MalObject],
    obj: &MalObject,
    env: &mut EnvironmentStack,
) -> std::result::Result<MalObject, LetError> {
    env.push();

    let bind = |(key, value): (&MalObject, &MalObject)| -> std::result::Result<(), LetError> {
        if let MalObject::Symbol(s) = key {
            eval(value, env)
                .map_err(|_| LetError::ValueEvaluationFailed)
                .map(|value| {
                    env.set(s.clone(), value);
                })
        } else {
            Err(LetError::BindToNonSymbol)
        }
    };

    let bind_result = bindings
        .iter()
        .tuples()
        .map(bind)
        .collect::<std::result::Result<Vec<()>, _>>();

    let let_result =
        bind_result.and_then(|_| eval(obj, env).map_err(|_| LetError::ValueEvaluationFailed));
    env.pop();
    let_result
}

fn evaluate_ast(ast: &MalObject, env: &mut EnvironmentStack) -> Result {
    log::debug!("eval_ast {:?}", ast);
    match ast {
        MalObject::Symbol(s) => fetch_symbol(s, env).map(|obj| obj.clone()),
        MalObject::List(list) => evaluate_sequence_elementwise(list, env).map(MalObject::List),
        MalObject::Vector(vec) => evaluate_sequence_elementwise(vec, env).map(MalObject::Vector),
        MalObject::Map(map) => evaluate_map(map, env),
        _ => Ok(ast.clone()),
    }
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
    env.get(s)
        .ok_or_else(|| Error::UnknownSymbol(s.name.clone()))
}