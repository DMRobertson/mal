use crate::environment::Environment;
use crate::types::{Closure, MalMap, MalObject, MalSymbol, PrimitiveFn};
use crate::{special_forms, types};
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

#[allow(non_snake_case)]
pub(crate) fn EVAL(ast: &MalObject, env: &Rc<Environment>) -> Result {
    use MalObject::List;
    match ast {
        List(list) => match list.len() {
            0 => Ok(MalObject::new_list()),
            _ => apply(list, env),
        },
        _ => evaluate_ast(ast, env),
    }
}

fn evaluate_ast(ast: &MalObject, env: &Rc<Environment>) -> Result {
    log::debug!("eval_ast {:?}", ast);
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

fn apply(argv: &[MalObject], env: &Rc<Environment>) -> Result {
    use MalObject::{Closure, Primitive, Symbol};
    log::debug!("apply {:?}", argv);
    if let Symbol(MalSymbol { name }) = &argv[0] {
        match name.as_str() {
            "def!" => return special_forms::apply_def(&argv[1..], env),
            "let*" => return special_forms::apply_let(&argv[1..], env),
            "do" => return special_forms::apply_do(&argv[1..], env),
            "if" => return special_forms::apply_if(&argv[1..], env),
            "fn*" => return special_forms::apply_fn(&argv[1..], env),
            _ => (),
        };
    };
    let evaluated = evaluate_sequence_elementwise(argv, env)?;
    let (callable, args) = evaluated.split_first().unwrap();
    match callable {
        Primitive(f) => call_primitive(f, args),
        Closure(f) => call_closure(f, args),
        _ => panic!("apply: bad MalObject {:?}", evaluated),
    }
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

fn call_closure(func: &Closure, args: &[MalObject]) -> Result {
    log::debug!("Call closure {:?} with {:?}", func, args);
    func.arity()
        .validate_for(args.len(), "closure")
        .map_err(Error::BadArgCount)?;
    let env = Environment::spawn_from(&func.parent);
    for (key, value) in func.parameters.iter().zip(args) {
        env.set(key.clone(), value.clone());
    }
    EVAL(&func.body, &env)
}
