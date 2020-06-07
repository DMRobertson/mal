use crate::environment::EnvironmentStack;
use crate::special_forms;
use crate::types::{MalMap, MalObject, MalSymbol};
use std::fmt;

pub type Result<T = MalObject> = std::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    UnknownSymbol(String),
    ListHeadNotSymbol,
    Def(special_forms::DefError),
    Let(special_forms::LetError),
}

pub type Evaluator = fn(&MalObject, &mut Context) -> Result;

pub struct Context<'a> {
    pub env: &'a mut EnvironmentStack,
    #[allow(non_snake_case)]
    pub evaluator: Evaluator,
}

impl<'a> Context<'a> {
    #[allow(non_snake_case)]
    pub(crate) fn EVAL(&mut self, obj: &MalObject) -> Result {
        (self.evaluator)(obj, self)
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
            Error::Def(e) => write!(f, "def!: {:?}", e),
            Error::Let(e) => write!(f, "let*: {:?}", e),
        }
    }
}

pub fn eval_ast_or_apply(
    ast: &MalObject,
    ctx: &mut Context,
    apply: fn(&[MalObject], &mut Context) -> Result,
) -> Result {
    use MalObject::List;
    match ast {
        List(list) => match list.len() {
            0 => Ok(List(MalList::new())),
            _ => apply(list, ctx),
        },
        _ => evaluate_ast(ast, ctx),
    }
}

fn evaluate_ast(ast: &MalObject, ctx: &mut Context) -> Result {
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
