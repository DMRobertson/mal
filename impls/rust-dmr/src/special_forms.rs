use crate::types::{truthy, Arity, MalObject};
use itertools::Itertools;

use crate::environment::Environment;
use crate::evaluator::{Error, Result, EVAL};
use std::rc::Rc;

#[derive(Debug)]
pub enum DefError {
    WrongArgCount(usize),
    KeyNotASymbol,
}

pub fn apply_def(args: &[MalObject], env: &Rc<Environment>) -> Result {
    let (key, value) = match args.len() {
        2 => Ok((&args[0], &args[1])),
        n => Err(Error::Def(DefError::WrongArgCount(n))),
    }?;
    let key = match key {
        MalObject::Symbol(s) => Ok(s),
        _ => Err(Error::Def(DefError::KeyNotASymbol)),
    }?;
    let value = EVAL(value, env)?;
    env.set(key.clone(), value.clone());
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

pub fn apply_let(args: &[MalObject], env: &Rc<Environment>) -> Result {
    use MalObject::{List, Vector};
    let (bindings, obj) = match args.len() {
        2 => Ok((&args[0], &args[1])),
        n => Err(Error::Let(LetError::WrongArgCount(n))),
    }?;
    match bindings {
        List(bindings) | Vector(bindings) if bindings.len() % 2 == 0 => {
            apply_let_evaluate(bindings, obj, env)
        }
        List(_) | Vector(_) => Err(Error::Let(LetError::BindingsOddLength)),
        _ => Err(Error::Let(LetError::BindingsNotSequence)),
    }
}

fn apply_let_evaluate(bindings: &[MalObject], obj: &MalObject, env: &Rc<Environment>) -> Result {
    let child = Rc::new(Environment::spawn_from(env));

    let bind = |(key, value): (&MalObject, &MalObject)| -> std::result::Result<(), Error> {
        if let MalObject::Symbol(s) = key {
            EVAL(value, &child).map(|value| {
                child.set(s.clone(), value);
            })
        } else {
            Err(Error::Let(LetError::BindToNonSymbol))
        }
    };

    let bind_result = bindings
        .iter()
        .tuples()
        .map(bind)
        .collect::<std::result::Result<Vec<()>, _>>();

    bind_result.and_then(|_| EVAL(obj, &child))
}

#[derive(Debug)]
pub enum DoError {
    NothingToDo,
}

pub fn apply_do(args: &[MalObject], env: &Rc<Environment>) -> Result {
    if args.is_empty() {
        return Err(Error::Do(DoError::NothingToDo));
    }
    let result: std::result::Result<Vec<MalObject>, _> =
        args.iter().map(|obj| EVAL(obj, env)).collect();
    // TODO returning a copy here---doesn't feel right
    Ok(result?.last().unwrap().clone())
}

pub fn apply_if(args: &[MalObject], env: &Rc<Environment>) -> Result {
    const ARITY: Arity = Arity::Between(2..=3);
    if !ARITY.contains(args.len()) {
        return Err(Error::BadArgCount("if", ARITY, args.len()));
    }
    let condition = EVAL(&args[0], env)?;
    if truthy(&condition) {
        EVAL(&args[1], env)
    } else if args.len() == 3 {
        EVAL(&args[2], env)
    } else {
        Ok(MalObject::Nil)
    }
}
