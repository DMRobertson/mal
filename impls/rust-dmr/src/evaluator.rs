use crate::environment::Environment;
use crate::types::{Closure, MalMap, MalObject, MalSymbol, PrimitiveFn};
use crate::{special_forms, types};
use std::borrow::Cow;
use std::fmt;
use std::rc::Rc;

pub type Result<T = MalObject> = std::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    UnknownSymbol(String),
    ListHeadNotSymbol,
    Def(special_forms::DefError),
    Let(special_forms::LetError),
    Do(special_forms::DoError),
    Fn(special_forms::FnError),
    TypeMismatch(types::TypeMismatch),
    BadArgCount(types::BadArgCount),
    DivideByZero,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Evaluation error: ")?;
        match self {
            Error::UnknownSymbol(s) => write!(f, "symbol {} not found", s),
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
        }
    }
}

pub(crate) type EvalContext = (MalObject, Rc<Environment>);

#[allow(non_snake_case)]
pub(crate) fn EVAL(orig_ast: &MalObject, orig_env: &Rc<Environment>) -> Result {
    use MalObject::List;
    use MalObject::{Closure, Primitive, Symbol};
    let mut ast = Cow::Borrowed(orig_ast);
    let mut env = Cow::Borrowed(orig_env);
    loop {
        match &*ast {
            List(argv) => match argv.len() {
                0 => return Ok(MalObject::new_list()),
                _ => {
                    log::debug!("apply {:?}", argv);
                    if let Symbol(MalSymbol { name }) = &argv[0] {
                        match name.as_str() {
                            "def!" => return special_forms::apply_def(&argv[1..], &env),
                            "let*" => {
                                let (new_ast, new_env) =
                                    special_forms::apply_let(&argv[1..], &env)?;
                                env = Cow::Owned(new_env);
                                ast = Cow::Owned(new_ast);
                                continue;
                            }
                            "do" => {
                                ast = Cow::Owned(special_forms::apply_do(&argv[1..], &env)?);
                                continue;
                            }
                            "if" => {
                                ast = Cow::Owned(special_forms::apply_if(&argv[1..], &env)?);
                                continue;
                            }
                            "fn*" => return special_forms::apply_fn(&argv[1..], &env),
                            // Any other initial symbol will be interpreted a a function call and handled below
                            _ => (),
                        };
                    };
                    let evaluated = evaluate_sequence_elementwise(&*argv, &env)?;
                    let (callable, args) = evaluated.split_first().unwrap();
                    match callable {
                        Primitive(f) => return call_primitive(f, args),
                        Closure(f) => {
                            env = Cow::Owned(make_closure_env(f, args)?);
                            ast = Cow::Owned(f.body.clone());
                            continue;
                        }
                        _ => panic!("apply: bad MalObject {:?}", evaluated),
                    }
                }
            },
            _ => return evaluate_ast(&ast, &env),
        };
    }
}

pub(crate) fn evaluate_ast(ast: &MalObject, env: &Rc<Environment>) -> Result {
    log::trace!("evaluate_ast {:?}", ast);
    match ast {
        MalObject::Symbol(s) => fetch_symbol(s, env),
        MalObject::List(list) => evaluate_sequence_elementwise(list, env).map(MalObject::wrap_list),
        MalObject::Vector(vec) => {
            evaluate_sequence_elementwise(vec, env).map(MalObject::wrap_vector)
        }
        MalObject::Map(map) => evaluate_map(map, env),
        _ => Ok(ast.clone()),
    }
}

fn evaluate_map(map: &MalMap, env: &Rc<Environment>) -> Result {
    let mut evaluated = MalMap::new();
    for key in map.keys() {
        let old_value = map.get(key).unwrap();
        let new_value = EVAL(old_value, env)?;
        evaluated.insert(key.clone(), new_value);
    }
    Ok(MalObject::Map(Rc::new(evaluated)))
}

pub fn evaluate_sequence_elementwise(
    seq: &[MalObject],
    env: &Rc<Environment>,
) -> std::result::Result<Vec<MalObject>, Error> {
    let mapped: std::result::Result<Vec<MalObject>, Error> =
        seq.iter().map(|obj| EVAL(obj, env)).collect();
    mapped
}

fn fetch_symbol(s: &MalSymbol, env: &Environment) -> Result {
    env.get(s)
        .ok_or_else(|| Error::UnknownSymbol(s.name.clone()))
}

pub fn call_primitive(func: &'static PrimitiveFn, args: &[MalObject]) -> Result {
    log::debug!("Call {} with {:?}", func.name, args);
    func.arity
        .validate_for(args.len(), func.name)
        .map_err(Error::BadArgCount)?;
    let result = (func.fn_ptr)(args);
    log::debug!("Call to {} resulted in {:?}", func.name, result);
    result
}

fn make_closure_env(func: &Closure, args: &[MalObject]) -> Result<Rc<Environment>> {
    log::debug!("Call closure {:?} with {:?}", func, args);
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
        let rest = rest.iter().map(|obj| obj.clone()).collect();
        env.set(rest_key.clone(), MalObject::wrap_list(rest));
    }
    Ok(env)
}
