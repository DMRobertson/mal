use crate::evaluator;
use crate::evaluator::Context;
use crate::types::{truthy, Arity, MalObject};
use itertools::Itertools;

use evaluator::Error;

#[derive(Debug)]
pub enum DefError {
    WrongArgCount(usize),
    KeyNotASymbol,
}

pub fn apply_def(args: &[MalObject], ctx: &mut Context) -> evaluator::Result {
    let (key, value) = match args.len() {
        2 => Ok((&args[0], &args[1])),
        n => Err(Error::Def(DefError::WrongArgCount(n))),
    }?;
    let key = match key {
        MalObject::Symbol(s) => Ok(s),
        _ => Err(Error::Def(DefError::KeyNotASymbol)),
    }?;
    let value = ctx.EVAL(value)?;
    ctx.env.set(key.clone(), value.clone());
    // Shouldn't this return a reference to the object in the map?
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

pub fn apply_let(args: &[MalObject], ctx: &mut Context) -> evaluator::Result {
    use MalObject::{List, Vector};
    let (bindings, obj) = match args.len() {
        2 => Ok((&args[0], &args[1])),
        n => Err(Error::Let(LetError::WrongArgCount(n))),
    }?;
    match bindings {
        List(bindings) | Vector(bindings) if bindings.len() % 2 == 0 => {
            apply_let_evaluate(bindings, obj, ctx)
        }
        List(_) | Vector(_) => Err(Error::Let(LetError::BindingsOddLength)),
        _ => Err(Error::Let(LetError::BindingsNotSequence)),
    }
}

fn apply_let_evaluate(
    bindings: &[MalObject],
    obj: &MalObject,
    ctx: &mut Context,
) -> evaluator::Result {
    ctx.env.push();

    let bind = |(key, value): (&MalObject, &MalObject)| -> Result<(), Error> {
        if let MalObject::Symbol(s) = key {
            ctx.EVAL(value).map(|value| {
                ctx.env.set(s.clone(), value);
            })
        } else {
            Err(Error::Let(LetError::BindToNonSymbol))
        }
    };

    let bind_result = bindings
        .iter()
        .tuples()
        .map(bind)
        .collect::<Result<Vec<()>, _>>();

    let let_result = bind_result.and_then(|_| ctx.EVAL(obj));
    ctx.env.pop();
    let_result
}

#[derive(Debug)]
pub enum DoError {
    NothingToDo,
}

pub fn apply_do(args: &[MalObject], ctx: &mut Context) -> evaluator::Result {
    if args.is_empty() {
        return Err(Error::Do(DoError::NothingToDo));
    }
    let result: Result<Vec<MalObject>, _> = args.iter().map(|obj| ctx.EVAL(obj)).collect();
    // TODO returning a copy here---doesn't feel right
    Ok(result?.last().unwrap().clone())
}

pub fn apply_if(args: &[MalObject], ctx: &mut Context) -> evaluator::Result {
    const ARITY: Arity = Arity::Between(2..=3);
    if !ARITY.contains(args.len()) {
        return Err(Error::BadArgCount("if", ARITY, args.len()));
    }
    let condition = ctx.EVAL(&args[0])?;
    if truthy(&condition) {
        ctx.EVAL(&args[1])
    } else if args.len() == 3 {
        ctx.EVAL(&args[2])
    } else {
        Ok(MalObject::Nil)
    }
}
