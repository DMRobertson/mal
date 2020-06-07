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
