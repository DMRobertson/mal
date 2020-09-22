use crate::types::{
    truthy, Arity, BadArgCount, BadClosureParameters, Closure, ClosureParameters, MalObject,
    MalSymbol, TypeMismatch,
};
use itertools::Itertools;

use crate::environment::Environment;
use crate::evaluator::{Error, ErrorDuringCatch, EvalContext, Result, EVAL};
use crate::special_forms::FnError::{BadVariadic, ParameterNotASymbol};
use std::rc::Rc;

#[derive(Debug)]
pub enum DefError {
    WrongArgCount(usize),
    KeyNotASymbol,
}

pub fn apply_def(args: &[MalObject], env: &Rc<Environment>, make_macro: bool) -> Result {
    let (key, value) = match args.len() {
        2 => Ok((&args[0], &args[1])),
        n => Err(Error::Def(DefError::WrongArgCount(n))),
    }?;
    let key = match key {
        MalObject::Symbol(s) => Ok(s),
        _ => Err(Error::Def(DefError::KeyNotASymbol)),
    }?;
    let value = EVAL(value, env)?;
    let value = match make_macro {
        true => match value {
            MalObject::Closure(c) => {
                let mut tweaked_closure = (*c).clone();
                tweaked_closure.is_macro = true;
                Ok(MalObject::Closure(Rc::new(tweaked_closure)))
            }
            _ => Err(Error::TypeMismatch(TypeMismatch::NotAClosure)),
        }?,
        false => value,
    };
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
    BadVariadic(BadClosureParameters),
}

pub fn apply_fn(args: &[MalObject], env: &Rc<Environment>) -> Result {
    // Start by checking that we've been given the right kind of arguments.
    // We expect exactly two arguments. The first, a parameters list, should be a
    // sequence of symbols. The second, the expression body of the function
    // we're defining, is any MalObject.
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
        is_macro: false,
        meta: MalObject::Nil,
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
    let ast_0_list = ast[0].as_seq().ok().filter(|ast0| !ast0.is_empty());
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
pub fn apply_try(args: &[MalObject], env: &Rc<Environment>) -> Result<EvalContext> {
    let ast = &args[0];
    let catch = args.get(1);
    let catch_data = catch.map(|obj| -> Result<_> {
        let data = &obj.as_list()?.payload;
        Arity::exactly(3)
            .validate_for(data.len(), "try* catch list")
            .map_err(Error::BadArgCount)?;

        let catch_sym: MalObject = MalObject::new_symbol("catch*");
        if data[0] != catch_sym {
            Err(Error::MissingCatchFromTry)?;
        };
        let exception_name = data[1].as_symbol()?;
        let exception_handler = &data[2];
        Ok((exception_name, exception_handler))
    });

    let catch_data = match catch_data {
        None => None,
        Some(Ok(x)) => Some(x),
        Some(Err(e)) => return Err(e),
    };

    match (EVAL(ast, env), catch_data) {
        (Ok(ast), _) => Ok((ast, env.clone())),
        (Err(original), None) => Err(original),
        (Err(original), Some((exception_name, exception_handler))) => {
            let exception_env = Environment::spawn_from(env);
            exception_env.set(exception_name.clone(), MalObject::from(&original));
            match EVAL(&exception_handler, &exception_env) {
                Ok(obj) => Ok((obj, env.clone())),
                Err(then) => Err(Error::InCatchHandler(ErrorDuringCatch {
                    original: Box::new(original),
                    then: Box::new(then),
                })),
            }
        }
    }
}
