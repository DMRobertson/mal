use crate::types::{
    truthy, Arity, BadArgCount, BadClosureParameters, Closure, ClosureParameters, MalList,
    MalObject, MalSymbol,
};
use itertools::Itertools;

use crate::environment::Environment;
use crate::evaluator::{Error, EvalContext, Result, EVAL};
use crate::special_forms::FnError::{BadVariadic, ParameterNotASymbol};
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

pub fn apply_let(args: &[MalObject], env: &Rc<Environment>) -> Result<EvalContext> {
    let (bindings, obj) = match args.len() {
        2 => Ok((&args[0], &args[1])),
        n => Err(Error::Let(LetError::WrongArgCount(n))),
    }?;
    let bindings = bindings
        .as_seq()
        .or(Err(Error::Let(LetError::BindingsNotSequence)))?;
    match bindings.len() % 2 == 0 {
        true => make_let_environment(bindings, env).and_then(|child| Ok((obj.clone(), child))),
        false => Err(Error::Let(LetError::BindingsOddLength)),
    }
}

fn make_let_environment(
    bindings: &[MalObject],
    parent: &Rc<Environment>,
) -> Result<Rc<Environment>> {
    let child = Environment::spawn_from(parent);

    let bind = |(key, value): (&MalObject, &MalObject)| -> std::result::Result<(), Error> {
        if let MalObject::Symbol(s) = key {
            // Note: evaluate in the child so that later bindings can refer to earlier ones
            EVAL(value, &child).map(|value| {
                child.set(s.clone(), value);
            })
        } else {
            Err(Error::Let(LetError::BindToNonSymbol))
        }
    };

    bindings
        .iter()
        .tuples()
        .map(bind)
        .collect::<std::result::Result<Vec<()>, _>>()?;
    Ok(child)
}

#[derive(Debug)]
pub enum DoError {
    NothingToDo,
}

pub fn apply_do(args: &[MalObject], env: &Rc<Environment>) -> Result<MalObject> {
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
        Ok(args[1].clone())
    } else if args.len() == 3 {
        Ok(args[2].clone())
    } else {
        Ok(MalObject::Nil)
    }
}

#[derive(Debug)]
pub enum FnError {
    WrongArgCount(usize),
    ParametersNotGivenAsList,
    ParameterNotASymbol,
    BadVariadic(BadClosureParameters),
}

pub fn apply_fn(args: &[MalObject], env: &Rc<Environment>) -> Result {
    // Start by checking that we've been given the right kind of arguments.
    // We expect exactly two arguments. The first, a parameters list, should be a sequence of symbols.
    // The second, the expression body of the function we're defining, is any MalObject.
    let (parameters, body) = match args.len() {
        2 => Ok((&args[0], &args[1])),
        n => Err(Error::Fn(FnError::WrongArgCount(n))),
    }?;
    let parameters = parameters
        .as_seq()
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
        parameters: ClosureParameters::new(parameters).map_err(|e| Error::Fn(BadVariadic(e)))?,
        body: body.clone(),
        parent: env.clone(),
    };
    Ok(MalObject::Closure(Rc::new(closure)))
}

pub(crate) fn apply_quasiquote(ast: &MalObject) -> std::result::Result<MalObject, BadArgCount> {
    match ast.as_seq().ok() {
        None => Ok(MalObject::wrap_list(vec![
            MalObject::new_symbol("quote"),
            ast.clone(),
        ])),
        Some(ast) => quasiquote_internal(ast),
    }
}

fn quasiquote_internal(ast: &[MalObject]) -> std::result::Result<MalObject, BadArgCount> {
    if ast.is_empty() {
        return Ok(MalObject::new_list());
    }
    Arity::at_least(1).validate_for(ast.len(), "quasiquote argument")?;
    let unquote: MalObject = MalObject::new_symbol("unquote");
    if ast[0] == unquote {
        Arity::exactly(2).validate_for(ast.len(), "unquote")?;
        return Ok(ast[1].clone());
    }

    let splice_unquote: MalObject = MalObject::new_symbol("splice-unquote");
    let ast_0_list = ast[0].as_seq().ok().filter(|ast0| ast0.len() >= 1);
    match ast_0_list {
        Some(ast_0_list) if ast_0_list[0] == splice_unquote => {
            Arity::at_least(1).validate_for(ast_0_list[1..].len(), "splice-unquote argument")?;
            let mut vec = Vec::new();
            vec.push(MalObject::new_symbol("concat"));
            vec.push(ast_0_list[1].clone());
            // TODO: can we avoid the recursion?
            vec.push(quasiquote_internal(&ast[1..])?);
            Ok(MalObject::wrap_list(vec))
        }
        _ => {
            let mut vec = Vec::new();
            vec.push(MalObject::new_symbol("cons"));
            // TODO: can we avoid the recursion?
            vec.push(apply_quasiquote(&ast[0])?);
            vec.push(quasiquote_internal(&ast[1..])?);
            Ok(MalObject::wrap_list(vec))
        }
    }
}
