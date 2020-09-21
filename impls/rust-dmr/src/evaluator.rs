use crate::environment::{Environment, UnknownSymbol};
use crate::evaluator::ApplyOutcome::EvaluateFurther;
use crate::types::{
    Arity, Closure, MalMap, MalObject, MalSymbol, PrimitiveEval, PrimitiveFnRef, TypeMismatch,
};
use crate::{environment, reader, special_forms, types};

use std::collections::HashMap;
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

pub type Result<T = MalObject> = std::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    UnknownSymbol(environment::UnknownSymbol),
    ListHeadNotSymbol,
    Def(special_forms::DefError),
    Let(special_forms::LetError),
    Do(special_forms::DoError),
    Fn(special_forms::FnError),
    MissingCatchFromTry,
    InCatchHandler(ErrorDuringCatch),
    TypeMismatch(types::TypeMismatch),
    BadArgCount(types::BadArgCount),
    BadIndex(isize, Range<usize>),
    DivideByZero,
    // TODO the arrangement of all these errors needs a rethink IMO!
    ReadError(reader::Error),
    IOError(std::io::Error),
    UserException(MalObject),
}

#[derive(Debug)]
pub struct ErrorDuringCatch {
    pub original: Box<Error>,
    pub then: Box<Error>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownSymbol(UnknownSymbol(s)) => write!(f, "'{}' not found", s),
            Error::ListHeadNotSymbol => {
                write!(f, "cannot apply list whose first entry is not a symbol")
            }
            Error::TypeMismatch(e) => write!(f, "type mismatch: {:?}", e),
            Error::Def(e) => write!(f, "def!: {:?}", e),
            Error::Let(e) => write!(f, "let*: {:?}", e),
            Error::Do(e) => write!(f, "do: {:?}", e),
            Error::Fn(e) => write!(f, "fn*: {:?}", e),
            Error::BadArgCount(e) => write!(f, "{}", e),
            Error::DivideByZero => write!(f, "cannot divide by zero!"),
            Error::ReadError(e) => write!(f, "read error: {}", e),
            Error::IOError(e) => write!(f, "io error: {}", e),
            Error::BadIndex(i, r) => {
                write!(f, "bad index: {} not in range [{}, {})", i, r.start, r.end)
            }
            Error::MissingCatchFromTry => write!(f, "bad syntax: missing catch* from try*"),
            Error::InCatchHandler(e) => write!(
                f,
                "{}\nWhile handling the above exception, another exception occurred: {}",
                e.original, e.then
            ),
            Error::UserException(e) => write!(f, "UserException: {}", e),
        }
    }
}

impl From<types::TypeMismatch> for Error {
    fn from(t: TypeMismatch) -> Self {
        Self::TypeMismatch(t)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IOError(e)
    }
}

impl From<&Error> for MalObject {
    fn from(e: &Error) -> Self {
        match e {
            Error::UserException(obj) => obj.clone(),
            _ => MalObject::String(format!("{}", e)),
        }
    }
}

pub(crate) type EvalContext = (MalObject, Rc<Environment>);

#[allow(non_snake_case)]
pub(crate) fn EVAL(orig_ast: &MalObject, orig_env: &Rc<Environment>) -> Result {
    use MalObject::{List, Symbol};
    let mut ast = orig_ast.clone();
    let mut env = orig_env.clone();
    loop {
        ast = macroexpand(&ast, &env)?;
        log::trace!("macroexpand produced {}", ast);
        match &ast {
            List(argv) => match argv.payload.len() {
                0 => return Ok(MalObject::new_list()),
                _ => {
                    log::trace!("apply ({})", &ast);
                    if let Symbol(name) = &argv.payload[0] {
                        match name.as_str() {
                            "def!" => {
                                let result =
                                    special_forms::apply_def(&argv.payload[1..], &env, false);
                                if let Ok(value) = &result {
                                    log::debug!("define {} as {}", argv.payload[1], value);
                                }
                                return result;
                            }
                            "defmacro!" => {
                                return special_forms::apply_def(&argv.payload[1..], &env, true)
                            }
                            "let*" => {
                                let (new_ast, new_env) =
                                    special_forms::apply_let(&argv.payload[1..], &env)?;
                                env = new_env;
                                ast = new_ast;
                                continue;
                            }
                            "do" => {
                                ast = special_forms::apply_do(&argv.payload[1..], &env)?;
                                continue;
                            }
                            "if" => {
                                ast = special_forms::apply_if(&argv.payload[1..], &env)?;
                                continue;
                            }
                            "fn*" => return special_forms::apply_fn(&argv.payload[1..], &env),
                            // Any other initial symbol will be interpreted a a function call and
                            // handled below
                            "quote" => {
                                Arity::exactly(1)
                                    .validate_for(argv.payload[1..].len(), "quote")
                                    .map_err(Error::BadArgCount)?;
                                return Ok(argv.payload[1].clone());
                            }
                            "quasiquote" => {
                                Arity::exactly(1)
                                    .validate_for(argv.payload[1..].len(), "quasiquote")
                                    .map_err(Error::BadArgCount)?;
                                ast = special_forms::apply_quasiquote(&argv.payload[1])
                                    .map_err(Error::BadArgCount)?;
                                continue;
                            }
                            "macroexpand" => {
                                Arity::exactly(1)
                                    .validate_for(argv.payload[1..].len(), "macroexpand")
                                    .map_err(Error::BadArgCount)?;
                                return macroexpand(&argv.payload[1], &env);
                            }
                            "try*" => {
                                Arity::Between(1..=2)
                                    .validate_for(argv.payload[1..].len(), "try*")
                                    .map_err(Error::BadArgCount)?;
                                let (new_ast, new_env) =
                                    special_forms::apply_try(&argv.payload[1..], &env)?;
                                env = new_env;
                                ast = new_ast;
                                continue;
                            }
                            _ => (),
                        };
                    };
                    let evaluated = evaluate_sequence_elementwise(&argv.payload, &env)?;
                    let (callable, args) = evaluated.split_first().unwrap();
                    match apply(callable, args)? {
                        ApplyOutcome::Finished(obj) => return Ok(obj),
                        ApplyOutcome::EvaluateFurther(next_ast, next_env) => {
                            ast = next_ast;
                            env = next_env;
                            continue;
                        }
                    }
                }
            },
            _ => return evaluate_ast(&ast, &env),
        };
    }
}

// Want to pull out the apply logic so we can use it in core::SWAP.
// We still want to allow EVAL above to use TCO, so we might return
// EvaluateFurther from apply in order to continue the EVAL loop. But this now
// means we might have to process EvaluateFuther in core::SWAP with a call to
// EVAL.

// Feels complex---maybe a premature optimisation?
// Depends how often closures need to EvaluateFurther.
// I'd imagine the point of lisp is that you want closures that can return calls
// to other closures, so fairly often?
pub(crate) enum ApplyOutcome {
    Finished(MalObject),
    EvaluateFurther(MalObject, Rc<Environment>),
}

// In order to apply a function we might have to apply that function, if the function is recursive.
// To avoid stack overflow, "apply" just does one step of the application and returns the next thing that needs evaluating.
// But that's only really useful within an EVAL call. Elsewhere we just want a value

// TODO: this all seems a bit suspicious, and I wonder if there something I've misunderstood here.
pub(crate) fn apply_fully(callable: &MalObject, args: &[MalObject]) -> Result {
    apply(callable, &args).and_then(|outcome| match outcome {
        ApplyOutcome::Finished(obj) => Ok(obj),
        ApplyOutcome::EvaluateFurther(ast, env) => EVAL(&ast, &env),
    })
}

pub(crate) fn apply(callable: &MalObject, args: &[MalObject]) -> Result<ApplyOutcome> {
    use MalObject::{Closure, Eval, Primitive};
    match callable {
        Primitive(f) => call_primitive(f, args).map(ApplyOutcome::Finished),
        Closure(f) => {
            let ast = f.body.clone();
            let env = make_closure_env(f, args)?;
            Ok(ApplyOutcome::EvaluateFurther(ast, env))
        }
        Eval(PrimitiveEval { env }) => {
            Arity::exactly(1)
                .validate_for(args.len(), "eval")
                .map_err(Error::BadArgCount)?;
            let env = env.upgrade().expect("eval: env destroyed");
            log::info!("Call from mal to EVAL with {}", args[0]);
            Ok(EvaluateFurther(args[0].clone(), env))
        }
        _ => Err(Error::TypeMismatch(TypeMismatch::NotCallable)),
    }
}

pub(crate) fn evaluate_ast(ast: &MalObject, env: &Rc<Environment>) -> Result {
    log::trace!("evaluate_ast {:?}", ast);
    match ast {
        MalObject::Symbol(s) => env.fetch(s).map_err(Error::UnknownSymbol),
        MalObject::List(list) => {
            evaluate_sequence_elementwise(&list.payload, env).map(MalObject::wrap_list)
        }
        MalObject::Vector(vec) => {
            evaluate_sequence_elementwise(&vec.payload, env).map(MalObject::wrap_vector)
        }
        MalObject::Map(map) => evaluate_map(map, env),
        _ => Ok(ast.clone()),
    }
}

fn evaluate_map(map: &MalMap, env: &Rc<Environment>) -> Result {
    let mut evaluated = HashMap::new();
    for (key, old_value) in map.payload.iter() {
        let new_value = EVAL(old_value, env)?;
        evaluated.insert(key.clone(), new_value);
    }
    Ok(MalObject::wrap_map(evaluated))
}

pub fn evaluate_sequence_elementwise(
    seq: &[MalObject],
    env: &Rc<Environment>,
) -> std::result::Result<Vec<MalObject>, Error> {
    let mapped: std::result::Result<Vec<MalObject>, Error> =
        seq.iter().map(|obj| EVAL(obj, env)).collect();
    mapped
}

pub(crate) fn pretty_print_args(args: &[MalObject]) -> String {
    match args.len() {
        0 => "no args".into(),
        1 => args[0].to_string(),
        _ => format!("\n\t{}", args.iter().join("\n\t")),
    }
}

pub fn call_primitive(func: &PrimitiveFnRef, args: &[MalObject]) -> Result {
    let func = func.payload;
    func.arity
        .validate_for(args.len(), func.name)
        .map_err(Error::BadArgCount)?;
    log::trace!("Call {} with {}", func.name, pretty_print_args(args));
    let result = (func.fn_ptr)(args);
    match &result {
        Ok(val) => log::trace!("Call to {} resulted in {}", func.name, val),
        Err(e) => log::trace!("Call to {} failed: {}", func.name, e),
    }
    result
}

fn make_closure_env(func: &Closure, args: &[MalObject]) -> Result<Rc<Environment>> {
    log::trace!("Call {} with {}", func, pretty_print_args(args));
    func.parameters
        .arity()
        .validate_for(args.len(), "closure")
        .map_err(Error::BadArgCount)?;
    let env = Environment::spawn_from(&func.parent);

    let (positional, rest) = args.split_at(func.parameters.positional.len());
    for (key, value) in func.parameters.positional.iter().zip(positional) {
        env.set(key.clone(), value.clone());
    }
    if let Some(rest_key) = &func.parameters.others {
        env.set(rest_key.clone(), MalObject::wrap_list(rest.to_vec()));
    }
    Ok(env)
}

fn is_macro_call<'a>(ast: &'a MalObject, env: &Environment) -> Option<&'a MalSymbol> {
    let symbol = ast
        .as_list()
        .ok()
        .filter(|seq| !seq.payload.is_empty())
        .map(|seq| seq.payload[0].as_symbol().ok())
        .flatten();
    let value = symbol.map(|sym| env.get(sym)).flatten();
    let is_macro = value
        .map(|obj| obj.as_closure().ok().map(|c| c.is_macro))
        .flatten()
        .unwrap_or(false);
    match is_macro {
        true => Some(symbol.unwrap()),
        false => None,
    }
}

fn macroexpand(ast: &MalObject, env: &Rc<Environment>) -> Result {
    let mut ast = ast.clone();
    let env = env.clone();
    while let Some(symbol) = is_macro_call(&ast, &env) {
        log::trace!("macroexpand: env={}", env);
        let closure = env.get(symbol).unwrap();
        ast = apply_fully(&closure, &ast.as_list().unwrap().payload[1..])?;
    }
    Ok(ast)
}
