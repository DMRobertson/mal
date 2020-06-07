use crate::environment::EnvironmentStack;
use crate::types::{MalMap, MalObject, MalSymbol};
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

pub type Evaluator = fn(&MalObject, &mut EnvironmentStack) -> Result;

pub struct Context<'a> {
    pub env: &'a mut EnvironmentStack,
    #[allow(non_snake_case)]
    pub evaluator: Evaluator,
}

impl<'a> Context<'a> {
    #[allow(non_snake_case)]
    fn EVAL(&mut self, obj: &MalObject) -> Result {
        (self.evaluator)(obj, &mut self.env)
    }
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

fn apply(argv: &[MalObject], ctx: &mut Context) -> Result {
    use MalObject::{Integer, PrimitiveBinaryOp, Symbol};
    log::debug!("apply {:?}", argv);
    if let Symbol(MalSymbol { name }) = &argv[0] {
        match name.as_str() {
            "def!" => return apply_def(&argv[1..], ctx).map_err(Error::DefError),
            "let*" => return apply_let(&argv[1..], ctx).map_err(Error::LetError),
            _ => (),
        };
    };
    let evaluated = evaluate_sequence_elementwise(argv, ctx)?;
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

fn apply_def(args: &[MalObject], ctx: &mut Context) -> std::result::Result<MalObject, DefError> {
    let (key, value) = match args.len() {
        2 => Ok((&args[0], &args[1])),
        n => Err(DefError::WrongArgCount(n)),
    }?;
    let key = match key {
        MalObject::Symbol(s) => Ok(s),
        _ => Err(DefError::KeyNotASymbol),
    }?;
    let value = ctx
        .EVAL(value)
        .map_err(|_| DefError::ValueEvaluationFailed)?;
    ctx.env.set(key.clone(), value.clone());
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

fn apply_let(args: &[MalObject], ctx: &mut Context) -> std::result::Result<MalObject, LetError> {
    use MalObject::{List, Vector};
    let (bindings, obj) = match args.len() {
        2 => Ok((&args[0], &args[1])),
        n => Err(LetError::WrongArgCount(n)),
    }?;
    match bindings {
        List(bindings) | Vector(bindings) if bindings.len() % 2 == 0 => {
            apply_let_evaluate(bindings, obj, ctx)
        }
        List(_) | Vector(_) => Err(LetError::BindingsOddLength),
        _ => Err(LetError::BindingsNotSequence),
    }
}

fn apply_let_evaluate(
    bindings: &[MalObject],
    obj: &MalObject,
    ctx: &mut Context,
) -> std::result::Result<MalObject, LetError> {
    ctx.env.push();

    let bind = |(key, value): (&MalObject, &MalObject)| -> std::result::Result<(), LetError> {
        if let MalObject::Symbol(s) = key {
            ctx.EVAL(value)
                .map_err(|_| LetError::ValueEvaluationFailed)
                .map(|value| {
                    ctx.env.set(s.clone(), value);
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
        bind_result.and_then(|_| ctx.EVAL(obj).map_err(|_| LetError::ValueEvaluationFailed));
    ctx.env.pop();
    let_result
}

pub fn evaluate_ast(ast: &MalObject, ctx: &mut Context) -> Result {
    log::debug!("eval_ast {:?}", ast);
    match ast {
        MalObject::Symbol(s) => fetch_symbol(s, &ctx.env).map(|obj| obj.clone()),
        MalObject::List(list) => evaluate_sequence_elementwise(list, ctx).map(MalObject::List),
        MalObject::Vector(vec) => evaluate_sequence_elementwise(vec, ctx).map(MalObject::Vector),
        MalObject::Map(map) => evaluate_map(map, ctx),
        _ => Ok(ast.clone()),
    }
}

fn evaluate_map(map: &MalMap, ctx: &mut Context) -> Result {
    let mut evaluated = MalMap::new();
    for key in map.keys() {
        let old_value = map.get(key).unwrap();
        let new_value = ctx.EVAL(old_value)?;
        evaluated.insert(key.clone(), new_value);
    }
    Ok(MalObject::Map(evaluated))
}

pub fn evaluate_sequence_elementwise(
    seq: &[MalObject],
    ctx: &mut Context,
) -> std::result::Result<Vec<MalObject>, Error> {
    let mapped: std::result::Result<Vec<MalObject>, Error> =
        seq.iter().map(|obj| ctx.EVAL(obj)).collect();
    mapped
}

fn fetch_symbol<'a>(s: &MalSymbol, env: &'a EnvironmentStack) -> Result<&'a MalObject> {
    env.get(s)
        .ok_or_else(|| Error::UnknownSymbol(s.name.clone()))
}
