use crate::evaluator::Context;
use crate::types::MalObject;
use itertools::Itertools;

#[derive(Debug)]
pub enum DefError {
    WrongArgCount(usize),
    KeyNotASymbol,
    ValueEvaluationFailed,
}

pub fn apply_def(
    args: &[MalObject],
    ctx: &mut Context,
) -> std::result::Result<MalObject, DefError> {
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

pub fn apply_let(
    args: &[MalObject],
    ctx: &mut Context,
) -> std::result::Result<MalObject, LetError> {
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
