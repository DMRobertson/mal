use crate::evaluator::{apply, ApplyOutcome};
use crate::types::{callable, Arity, Atom, MalInt, MalObject, PrimitiveFn, TypeMismatch};
use crate::{evaluator, printer, reader, types};
use itertools::Itertools;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::read_to_string;

fn grab_ints(args: &[MalObject]) -> evaluator::Result<Vec<MalInt>> {
    let type_check: Result<Vec<_>, _> = args.iter().map(MalInt::try_from).collect();
    type_check.map_err(evaluator::Error::TypeMismatch)
}

fn sum_(args: &[MalObject]) -> evaluator::Result {
    let value = grab_ints(args)?
        .iter()
        .fold(0 as MalInt, |acc, &x| acc.wrapping_add(x));
    Ok(MalObject::Integer(value))
}
const SUM: PrimitiveFn = PrimitiveFn {
    name: "+",
    fn_ptr: sum_,
    arity: Arity::AtLeast(0..),
};
fn sub_(args: &[MalObject]) -> evaluator::Result {
    match grab_ints(args)?.as_slice() {
        [x, y] => Ok(MalObject::Integer(x.wrapping_sub(*y))),
        _ => panic!(),
    }
}
const SUB: PrimitiveFn = PrimitiveFn {
    name: "-",
    fn_ptr: sub_,
    arity: Arity::exactly(2),
};

fn mul_(args: &[MalObject]) -> evaluator::Result {
    let value = grab_ints(args)?
        .iter()
        .fold(1 as MalInt, |acc, &x| acc.wrapping_mul(x));
    Ok(MalObject::Integer(value))
}

const MUL: PrimitiveFn = PrimitiveFn {
    name: "*",
    fn_ptr: mul_,
    arity: Arity::AtLeast(0..),
};

fn div_(args: &[MalObject]) -> evaluator::Result {
    match grab_ints(args)?.as_slice() {
        [_, 0] => Err(evaluator::Error::DivideByZero),
        [x, y] => Ok(MalObject::Integer(x.wrapping_div(*y))),
        _ => panic!(),
    }
}
const DIV: PrimitiveFn = PrimitiveFn {
    name: "/",
    fn_ptr: div_,
    arity: Arity::exactly(2),
};

fn comparison_(args: &[MalObject], comp: fn(&MalInt, &MalInt) -> bool) -> evaluator::Result {
    match grab_ints(args)?.as_slice() {
        [x, y] => Ok(MalObject::Bool(comp(x, y))),
        _ => panic!(),
    }
}

// TODO I don't know if this really justifies a macro... but it was an
// interesting learning experience!
macro_rules! comparison_primitive {
    ($SYMBOL:tt, $NAME:ident) => {
        paste::item! {
            const $NAME: PrimitiveFn = PrimitiveFn {
                name: stringify!($SYMBOL),
                fn_ptr: |args: &[MalObject]| comparison_(args, MalInt:: [<$NAME:lower>]),
                arity: Arity::exactly(2),
            };
        }
    };
}

comparison_primitive!(<, LT);
comparison_primitive!(<=, LE);
comparison_primitive!(>, GT);
comparison_primitive!(>=, GE);

fn list_(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::wrap_list(args.to_vec()))
}

const LIST: PrimitiveFn = PrimitiveFn {
    name: "list",
    fn_ptr: list_,
    arity: Arity::at_least(0),
};

fn list_test_(args: &[MalObject]) -> evaluator::Result {
    let is_list = match args[0] {
        MalObject::List(_) => true,
        _ => false,
    };
    Ok(MalObject::Bool(is_list))
}

const LIST_TEST: PrimitiveFn = PrimitiveFn {
    name: "list?",
    fn_ptr: list_test_,
    arity: Arity::exactly(1),
};

fn empty_test_(args: &[MalObject]) -> evaluator::Result {
    match &args[0] {
        MalObject::List(list) => Ok(list.is_empty()),
        MalObject::Vector(vec) => Ok(vec.is_empty()),
        _ => Err(evaluator::Error::TypeMismatch(
            types::TypeMismatch::NotASequence,
        )),
    }
    .map(MalObject::Bool)
}

const EMPTY_TEST: PrimitiveFn = PrimitiveFn {
    name: "empty?",
    fn_ptr: empty_test_,
    arity: Arity::exactly(1),
};

fn count_(args: &[MalObject]) -> evaluator::Result {
    match &args[0] {
        MalObject::List(list) => Ok(list.len() as MalInt),
        MalObject::Vector(vec) => Ok(vec.len() as MalInt),
        MalObject::Nil => Ok(0 as MalInt),
        _ => Err(evaluator::Error::TypeMismatch(
            // TODO better error here!
            types::TypeMismatch::NotASequence,
        )),
    }
    .map(MalObject::Integer)
}

const COUNT: PrimitiveFn = PrimitiveFn {
    name: "count",
    fn_ptr: count_,
    arity: Arity::exactly(1),
};

fn equal(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(equal_(args)))
}

fn equal_(args: &[MalObject]) -> bool {
    use MalObject::*;
    match &args[..2] {
        [Integer(x), Integer(y)] => x == y,
        [Bool(x), Bool(y)] => x == y,
        [List(x), List(y)]
        | [List(x), Vector(y)]
        | [Vector(x), List(y)]
        | [Vector(x), Vector(y)] => equal_sequences(x, y),
        [String(x), String(y)] => x == y,
        [Keyword(x), Keyword(y)] => x == y,
        [Nil, Nil] => true,
        [_, _] => false,
        _ => unreachable!(),
    }
}

// TODO Something very wrong here---shouldn't be cloning. I think the
// PrimitiveFns should be taking their args as refs! But let's get it working
// first.
// Update: Think this is fine since MalObject should be cheap to clone?
fn equal_sequences(xs: &[MalObject], ys: &[MalObject]) -> bool {
    xs.len() == ys.len()
        && xs
            .iter()
            .zip(ys)
            .all(|(x, y)| equal_(&[x.clone(), y.clone()]))
}

const EQUAL: PrimitiveFn = PrimitiveFn {
    name: "=",
    fn_ptr: equal,
    arity: Arity::exactly(2),
};

fn print_string_internal(
    args: &[MalObject],
    mode: printer::PrintMode,
    sep: &'static str,
    to_screen: bool,
) -> evaluator::Result {
    let text = args.iter().map(|arg| printer::pr_str(arg, mode)).join(sep);
    if to_screen {
        // TODO bypassing the "interface" in cmdline.rs. Maybe that's fine?
        println!("{}", text);
        Ok(MalObject::Nil)
    } else {
        Ok(MalObject::String(text))
    }
}

const PR_STR: PrimitiveFn = PrimitiveFn {
    name: "pr-str",
    fn_ptr: |args| {
        print_string_internal(args, printer::PrintMode::ReadableRepresentation, " ", false)
    },
    arity: Arity::at_least(0),
};

const STR: PrimitiveFn = PrimitiveFn {
    name: "str",
    fn_ptr: |args| print_string_internal(args, printer::PrintMode::Directly, "", false),
    arity: Arity::at_least(0),
};

const PRN: PrimitiveFn = PrimitiveFn {
    name: "prn",
    fn_ptr: |args| {
        print_string_internal(args, printer::PrintMode::ReadableRepresentation, " ", true)
    },
    arity: Arity::at_least(0),
};

const PRINTLN: PrimitiveFn = PrimitiveFn {
    name: "println",
    fn_ptr: |args| print_string_internal(args, printer::PrintMode::Directly, " ", true),
    arity: Arity::at_least(0),
};

const READ_STRING: PrimitiveFn = PrimitiveFn {
    name: "read-string",
    fn_ptr: read_string_,
    arity: Arity::exactly(1),
};
fn read_string_(args: &[MalObject]) -> evaluator::Result {
    match &args[0] {
        MalObject::String(s) => reader::read_str(s).map_err(evaluator::Error::ReadError),
        _ => Err(evaluator::Error::TypeMismatch(
            types::TypeMismatch::NotAString,
        )),
    }
}

const SLURP: PrimitiveFn = PrimitiveFn {
    name: "slurp",
    fn_ptr: slurp_,
    arity: Arity::exactly(1),
};
fn slurp_(args: &[MalObject]) -> evaluator::Result {
    match &args[0] {
        MalObject::String(s) => read_to_string(s).map_err(evaluator::Error::IOError),
        _ => Err(evaluator::Error::TypeMismatch(
            types::TypeMismatch::NotAString,
        )),
    }
    .map(MalObject::String)
}

const ATOM: PrimitiveFn = PrimitiveFn {
    name: "atom",
    fn_ptr: atom_,
    arity: Arity::exactly(1),
};
fn atom_(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Atom(Atom::new(&args[0])))
}
const ATOM_TEST: PrimitiveFn = PrimitiveFn {
    name: "atom?",
    fn_ptr: atom_test_,
    arity: Arity::exactly(1),
};
fn atom_test_(args: &[MalObject]) -> evaluator::Result {
    match &args[0] {
        MalObject::Atom(_) => Ok(MalObject::Bool(true)),
        _ => Ok(MalObject::Bool(false)),
    }
}

const DEREF: PrimitiveFn = PrimitiveFn {
    name: "deref",
    fn_ptr: deref_,
    arity: Arity::exactly(1),
};
fn deref_(args: &[MalObject]) -> evaluator::Result {
    match &args[0] {
        MalObject::Atom(a) => Ok(a.clone_payload()),
        _ => Err(evaluator::Error::TypeMismatch(TypeMismatch::NotAnAtom)),
    }
}

const RESET: PrimitiveFn = PrimitiveFn {
    name: "reset!",
    fn_ptr: reset_,
    arity: Arity::exactly(2),
};
fn reset_(args: &[MalObject]) -> evaluator::Result {
    match args {
        [MalObject::Atom(a), obj] => {
            a.replace(obj);
            Ok(obj.clone())
        }
        [_, _] => Err(evaluator::Error::TypeMismatch(TypeMismatch::NotAnAtom)),
        _ => unreachable!(),
    }
}

const SWAP: PrimitiveFn = PrimitiveFn {
    name: "swap!",
    fn_ptr: swap_,
    arity: Arity::at_least(2),
};
fn swap_(swap_args: &[MalObject]) -> evaluator::Result {
    use MalObject::Atom;
    match &swap_args[..2] {
        [Atom(a), f] if callable(f) => {
            let args = {
                let mut args = Vec::new();
                args.push(a.clone_payload());
                args.extend_from_slice(&swap_args[2..]);
                args
            };
            let result = apply(f, &args);
            let obj = result.and_then(|outcome| match outcome {
                ApplyOutcome::Finished(obj) => Ok(obj),
                ApplyOutcome::EvaluateFurther(ast, env) => evaluator::EVAL(&ast, &env),
            })?;
            a.replace(&obj);
            Ok(obj)
        }
        [Atom(_), _] => Err(evaluator::Error::TypeMismatch(TypeMismatch::NotCallable)),
        _ => unreachable!(),
    }
}

const CONS: PrimitiveFn = PrimitiveFn {
    name: "cons",
    fn_ptr: cons_,
    arity: Arity::exactly(2),
};
fn cons_(args: &[MalObject]) -> evaluator::Result {
    match args {
        [head, MalObject::List(tail)]
        //| [head, MalObject::Vector(tail)]
         => {
            let mut elements = Vec::new();
            elements.push(head.clone());
            elements.extend(tail.iter().map(MalObject::clone));
            Ok(MalObject::wrap_list(elements))
        }
        [_, _] => Err(evaluator::Error::TypeMismatch(TypeMismatch::NotASequence)),
        _ => unreachable!(),
    }
}

const CONCAT: PrimitiveFn = PrimitiveFn {
    name: "concat",
    fn_ptr: concat_,
    arity: Arity::at_least(0),
};
fn concat_(args: &[MalObject]) -> evaluator::Result {
    let mut output = Vec::new();
    let mut extend = |seq: &MalObject| match seq {
        MalObject::List(elements) | MalObject::Vector(elements) => {
            output.extend(elements.iter().map(MalObject::clone));
            Ok(())
        }
        _ => Err(evaluator::Error::TypeMismatch(TypeMismatch::NotASequence)),
    };
    for arg in args {
        extend(arg)?;
    }
    Ok(MalObject::wrap_list(output))
}

type Namespace = HashMap<&'static str, &'static PrimitiveFn>;
lazy_static! {
    pub static ref CORE: Namespace = {
        let mut map = Namespace::new();
        for func in &[
            SUM,
            SUB,
            MUL,
            DIV,
            LIST,
            LIST_TEST,
            EMPTY_TEST,
            COUNT,
            GT,
            GE,
            LT,
            LE,
            EQUAL,
            PR_STR,
            STR,
            PRN,
            PRINTLN,
            READ_STRING,
            SLURP,
            ATOM,
            ATOM_TEST,
            DEREF,
            RESET,
            SWAP,
            CONS,
            CONCAT,
        ] {
            map.insert(func.name, func);
        }
        map
    };
}