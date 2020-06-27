use crate::types::{truthy, Arity, Closure, MalList, MalObject, MalSymbol};
use itertools::Itertools;

use crate::environment::Environment;
use crate::evaluator::{Error, Result, EVAL};
use crate::special_forms::FnError::ParameterNotASymbol;
use std::convert::TryFrom;
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
    let child = Environment::spawn_from(env);

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
    Arity::Between(2..=3)
        .validate_for(args.len(), "if")
        .map_err(Error::BadArgCount)?;
    let condition = EVAL(&args[0], env)?;
    if truthy(&condition) {
        EVAL(&args[1], env)
    } else if args.len() == 3 {
        EVAL(&args[2], env)
    } else {
        Ok(MalObject::Nil)
    }
}

#[derive(Debug)]
pub enum FnError {
    WrongArgCount(usize),
    ParametersNotGivenAsList,
    ParameterNotASymbol,
}

pub fn apply_fn(args: &[MalObject], env: &Rc<Environment>) -> Result {
    let (parameters, body) = match args.len() {
        2 => Ok((&args[0], &args[1])),
        n => Err(Error::Fn(FnError::WrongArgCount(n))),
    }?;
    let parameters = Rc::<MalList>::try_from(parameters)
        .or(Err(FnError::ParametersNotGivenAsList))
        .map_err(Error::Fn)?;
    let extract_symbol = |obj: &MalObject| match obj {
        MalObject::Symbol(s) => Ok(s.clone()),
        _ => Err(ParameterNotASymbol),
    };
    let parameters: std::result::Result<Vec<MalSymbol>, _> =
        parameters.iter().map(extract_symbol).collect();
    let parameters = parameters.map_err(Error::Fn)?;
    let closure = Closure {
        parameters,
        body: body.clone(),
        parent: env.clone(),
    };
    Ok(MalObject::Closure(Rc::new(closure)))
}
