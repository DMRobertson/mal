use crate::types::{
    callable, Arity, Atom, HashKey, MalInt, MalObject, MapError, PrimitiveFn, TypeMismatch,
};
use crate::{evaluator, printer, reader, types};
use itertools::Itertools;
use linefeed::{DefaultTerminal, Interface, ReadResult};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::read_to_string;
use std::time::SystemTime;

fn grab_ints(args: &[MalObject]) -> evaluator::Result<Vec<MalInt>> {
    let type_check: Result<Vec<_>, _> = args.iter().map(|o| o.as_int()).collect();
    type_check.map_err(evaluator::Error::TypeMismatch)
}

const SUM: PrimitiveFn = PrimitiveFn {
    name: "+",
    fn_ptr: sum_,
    arity: Arity::AtLeast(0..),
};

fn sum_(args: &[MalObject]) -> evaluator::Result {
    let value = grab_ints(args)?
        .iter()
        .fold(0 as MalInt, |acc, &x| acc.wrapping_add(x));
    Ok(MalObject::Integer(value))
}

const SUB: PrimitiveFn = PrimitiveFn {
    name: "-",
    fn_ptr: sub_,
    arity: Arity::exactly(2),
};

fn sub_(args: &[MalObject]) -> evaluator::Result {
    match grab_ints(args)?.as_slice() {
        [x, y] => Ok(MalObject::Integer(x.wrapping_sub(*y))),
        _ => panic!(),
    }
}

const MUL: PrimitiveFn = PrimitiveFn {
    name: "*",
    fn_ptr: mul_,
    arity: Arity::AtLeast(0..),
};

fn mul_(args: &[MalObject]) -> evaluator::Result {
    let value = grab_ints(args)?
        .iter()
        .fold(1 as MalInt, |acc, &x| acc.wrapping_mul(x));
    Ok(MalObject::Integer(value))
}

const DIV: PrimitiveFn = PrimitiveFn {
    name: "/",
    fn_ptr: div_,
    arity: Arity::exactly(2),
};

fn div_(args: &[MalObject]) -> evaluator::Result {
    match grab_ints(args)?.as_slice() {
        [_, 0] => Err(evaluator::Error::DivideByZero),
        [x, y] => Ok(MalObject::Integer(x.wrapping_div(*y))),
        _ => unreachable!(),
    }
}

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

const LIST: PrimitiveFn = PrimitiveFn {
    name: "list",
    fn_ptr: list_,
    arity: Arity::at_least(0),
};

fn list_(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::wrap_list(args.to_vec()))
}

const LIST_TEST: PrimitiveFn = PrimitiveFn {
    name: "list?",
    fn_ptr: list_test_,
    arity: Arity::exactly(1),
};

fn list_test_(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(args[0].is_list()))
}

const VECTOR: PrimitiveFn = PrimitiveFn {
    name: "vector",
    fn_ptr: vector_,
    arity: Arity::at_least(0),
};

fn vector_(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::wrap_vector(args.to_vec()))
}

const VECTOR_TEST: PrimitiveFn = PrimitiveFn {
    name: "vector?",
    fn_ptr: vector_test_,
    arity: Arity::exactly(1),
};

fn vector_test_(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(args[0].is_vector()))
}

const SEQUENTIAL_TEST: PrimitiveFn = PrimitiveFn {
    name: "sequential?",
    fn_ptr: sequential_test_,
    arity: Arity::exactly(1),
};

fn sequential_test_(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(args[0].is_seq()))
}

const EMPTY_TEST: PrimitiveFn = PrimitiveFn {
    name: "empty?",
    fn_ptr: empty_test_,
    arity: Arity::exactly(1),
};

fn empty_test_(args: &[MalObject]) -> evaluator::Result {
    args[0]
        .as_seq()
        .map(|slice| slice.is_empty())
        .map(MalObject::Bool)
        .map_err(evaluator::Error::TypeMismatch)
}

const COUNT: PrimitiveFn = PrimitiveFn {
    name: "count",
    fn_ptr: count_,
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

const EQUAL: PrimitiveFn = PrimitiveFn {
    name: "=",
    fn_ptr: equal,
    arity: Arity::exactly(2),
};

fn equal(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(args[0] == args[1]))
}

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
    let string = args[0]
        .as_string()
        .map_err(evaluator::Error::TypeMismatch)?;
    reader::read_str(string).map_err(evaluator::Error::ReadError)
}

const SLURP: PrimitiveFn = PrimitiveFn {
    name: "slurp",
    fn_ptr: slurp_,
    arity: Arity::exactly(1),
};

fn slurp_(args: &[MalObject]) -> evaluator::Result {
    let s = args[0]
        .as_string()
        .map_err(evaluator::Error::TypeMismatch)?;
    read_to_string(s)
        .map_err(evaluator::Error::IOError)
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
    Ok(MalObject::Bool(args[0].is_atom()))
}

const DEREF: PrimitiveFn = PrimitiveFn {
    name: "deref",
    fn_ptr: deref_,
    arity: Arity::exactly(1),
};

fn deref_(args: &[MalObject]) -> evaluator::Result {
    args[0]
        .as_atom()
        .map_err(evaluator::Error::TypeMismatch)
        .map(Atom::clone_payload)
}

const RESET: PrimitiveFn = PrimitiveFn {
    name: "reset!",
    fn_ptr: reset_,
    arity: Arity::exactly(2),
};

fn reset_(args: &[MalObject]) -> evaluator::Result {
    let atom = args[0].as_atom().map_err(evaluator::Error::TypeMismatch)?;
    atom.replace(&args[1]);
    Ok(args[1].clone())
}

const SWAP: PrimitiveFn = PrimitiveFn {
    name: "swap!",
    fn_ptr: swap_,
    arity: Arity::at_least(2),
};

fn swap_(swap_args: &[MalObject]) -> evaluator::Result {
    let atom = swap_args[0]
        .as_atom()
        .map_err(evaluator::Error::TypeMismatch)?;

    let f = &swap_args[1];
    if !callable(f) {
        return Err(evaluator::Error::TypeMismatch(TypeMismatch::NotCallable));
    }
    let args = {
        let mut args = Vec::new();
        args.push(atom.clone_payload());
        args.extend_from_slice(&swap_args[2..]);
        args
    };
    let obj = evaluator::apply_fully(f, &args)?;
    atom.replace(&obj);
    Ok(obj)
}

const CONS: PrimitiveFn = PrimitiveFn {
    name: "cons",
    fn_ptr: cons_,
    arity: Arity::exactly(2),
};

fn cons_(args: &[MalObject]) -> evaluator::Result {
    let head = &args[0];
    let tail = args[1].as_seq().map_err(evaluator::Error::TypeMismatch)?;

    let mut elements = Vec::new();
    elements.push(head.clone());
    elements.extend(tail.iter().map(MalObject::clone));
    Ok(MalObject::wrap_list(elements))
}

const CONCAT: PrimitiveFn = PrimitiveFn {
    name: "concat",
    fn_ptr: concat_,
    arity: Arity::at_least(0),
};

fn concat_(args: &[MalObject]) -> evaluator::Result {
    let mut output = Vec::new();
    let mut extend = |obj: &MalObject| {
        obj.as_seq()
            .map(|elements| output.extend(elements.iter().map(MalObject::clone)))
            .map_err(evaluator::Error::TypeMismatch)
    };

    for arg in args {
        extend(arg)?;
    }
    Ok(MalObject::wrap_list(output))
}

const NTH: PrimitiveFn = PrimitiveFn {
    name: "nth",
    fn_ptr: nth_,
    arity: Arity::exactly(2),
};

fn nth_(args: &[MalObject]) -> evaluator::Result {
    let seq = args[0].as_seq().map_err(evaluator::Error::TypeMismatch)?;
    let orig_index = args[1].as_int().map_err(evaluator::Error::TypeMismatch)?;
    nth_internal(seq, orig_index)
}

fn nth_internal(seq: &[MalObject], orig_index: isize) -> evaluator::Result {
    let index = usize::try_from(orig_index).ok();
    let value = index
        .map(|index| seq.get(index))
        .flatten()
        .map(MalObject::clone);

    value.ok_or_else(|| evaluator::Error::BadIndex(orig_index, 0..seq.len()))
}

const FIRST: PrimitiveFn = PrimitiveFn {
    name: "first",
    fn_ptr: first_,
    arity: Arity::exactly(1),
};

fn first_(args: &[MalObject]) -> evaluator::Result {
    if args[0].is_nil() {
        return Ok(MalObject::Nil);
    }
    let seq = args[0].as_seq().map_err(evaluator::Error::TypeMismatch)?;
    match seq.is_empty() {
        true => Ok(MalObject::Nil),
        false => nth_internal(seq, 0),
    }
}

const REST: PrimitiveFn = PrimitiveFn {
    name: "rest",
    fn_ptr: rest_,
    arity: Arity::exactly(1),
};

fn rest_(args: &[MalObject]) -> evaluator::Result {
    if args[0].is_nil() {
        return Ok(MalObject::new_list());
    }
    let seq = args[0].as_seq().map_err(evaluator::Error::TypeMismatch)?;
    if seq.is_empty() {
        return Ok(MalObject::new_list());
    }
    // Feels a shame to make the copy here. Maybe we could have a
    // MalObject::ListSlice which appears like a list to the outside world, but
    // internally is a view into an list owned elsewhere?
    let copied = seq[1..].iter().map(MalObject::clone).collect();
    Ok(MalObject::wrap_list(copied))
}

const SYMBOL: PrimitiveFn = PrimitiveFn {
    name: "symbol",
    fn_ptr: symbol_,
    arity: Arity::exactly(1),
};

fn symbol_(args: &[MalObject]) -> evaluator::Result {
    args[0]
        .as_string()
        .map_err(evaluator::Error::TypeMismatch)
        .map(MalObject::new_symbol)
}

const SYMBOL_TEST: PrimitiveFn = PrimitiveFn {
    name: "symbol?",
    fn_ptr: symbol_test_,
    arity: Arity::exactly(1),
};

fn symbol_test_(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(args[0].is_symbol()))
}

const KEYWORD: PrimitiveFn = PrimitiveFn {
    name: "keyword",
    fn_ptr: keyword_,
    arity: Arity::exactly(1),
};

fn keyword_(args: &[MalObject]) -> evaluator::Result {
    match &args[0] {
        MalObject::String(s) => Ok(MalObject::new_keyword(s)),
        MalObject::Keyword(_) => Ok(args[0].clone()),
        _ => Err(evaluator::Error::TypeMismatch(TypeMismatch::NotIntoKeyword)),
    }
}

const KEYWORD_TEST: PrimitiveFn = PrimitiveFn {
    name: "keyword?",
    fn_ptr: keyword_test_,
    arity: Arity::exactly(1),
};

fn keyword_test_(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(args[0].is_keyword()))
}

const MAP_TEST: PrimitiveFn = PrimitiveFn {
    name: "map?",
    fn_ptr: map_test_,
    arity: Arity::exactly(1),
};

fn map_test_(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(args[0].is_map()))
}

const NIL_TEST: PrimitiveFn = PrimitiveFn {
    name: "nil?",
    fn_ptr: nil_test_,
    arity: Arity::exactly(1),
};
fn nil_test_(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(args[0].is_nil()))
}
const TRUE_TEST: PrimitiveFn = PrimitiveFn {
    name: "true?",
    fn_ptr: true_test_,
    arity: Arity::exactly(1),
};
fn true_test_(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(args[0].as_bool().unwrap_or(false)))
}
const FALSE_TEST: PrimitiveFn = PrimitiveFn {
    name: "false?",
    fn_ptr: false_test_,
    arity: Arity::exactly(1),
};
fn false_test_(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(
        args[0].as_bool().map(|b| !b).unwrap_or(false),
    ))
}

const APPLY: PrimitiveFn = PrimitiveFn {
    name: "apply",
    fn_ptr: apply_,
    arity: Arity::at_least(2),
};
fn apply_(args: &[MalObject]) -> evaluator::Result {
    let mut concatenated = args[1..args.len() - 1].to_vec();
    let last = args[args.len() - 1].as_seq()?;
    concatenated.extend_from_slice(last);
    evaluator::apply_fully(&args[0], &concatenated)
}

const MAP: PrimitiveFn = PrimitiveFn {
    name: "map",
    fn_ptr: map_,
    arity: Arity::exactly(2),
};
fn map_(args: &[MalObject]) -> evaluator::Result {
    let result: Result<Vec<_>, _> = args[1]
        .as_seq()?
        .chunks_exact(1)
        .map(|obj| evaluator::apply_fully(&args[0], obj))
        .collect();
    Ok(MalObject::wrap_list(result?))
}

const SEQ: PrimitiveFn = PrimitiveFn {
    name: "seq",
    fn_ptr: seq_,
    arity: Arity::exactly(1),
};
fn seq_(args: &[MalObject]) -> evaluator::Result {
    use MalObject::*;
    match &args[0] {
        String(x) if x.is_empty() => return Ok(Nil),
        List(x) if x.payload.is_empty() => return Ok(Nil),
        Vector(x) if x.payload.is_empty() => return Ok(Nil),
        _ => {}
    }

    match &args[0] {
        Nil => Ok(Nil),
        String(s) => Ok(MalObject::wrap_list(
            s.chars()
                .map(|substr| MalObject::String(substr.to_string()))
                .collect(),
        )),
        List(_) => Ok(args[0].clone()),
        Vector(x) => Ok(MalObject::wrap_list(x.0.clone())),
        _ => Err(evaluator::Error::TypeMismatch(TypeMismatch::NotASequence)),
    }
}

const CONJ: PrimitiveFn = PrimitiveFn {
    name: "conj",
    fn_ptr: conj_,
    arity: Arity::at_least(2),
};
fn conj_(args: &[MalObject]) -> evaluator::Result {
    let old = args[0].as_seq()?;
    let new = &args[1..];
    match &args[0] {
        MalObject::List(_) => {
            let mut result = new.to_vec();
            result.reverse();
            result.extend_from_slice(old);
            Ok(MalObject::wrap_list(result))
        }
        MalObject::Vector(_) => {
            let mut result = old.to_vec();
            result.extend_from_slice(new);
            Ok(MalObject::wrap_vector(result))
        }
        _ => unreachable!(),
    }
}

const HASH_MAP: PrimitiveFn = PrimitiveFn {
    name: "hash-map",
    fn_ptr: hash_map,
    arity: Arity::Even,
};

fn hash_map(args: &[MalObject]) -> evaluator::Result {
    types::build_map(args.to_owned()).map_err(|e| match e {
        MapError::MissingValue => unreachable!(), // parity checked by PrimtiveFn
        MapError::UnhashableKey => evaluator::Error::TypeMismatch(TypeMismatch::NotAValidKey),
    })
}

const ASSOC: PrimitiveFn = PrimitiveFn {
    name: "assoc",
    fn_ptr: assoc_,
    arity: Arity::Odd,
};
fn assoc_(args: &[MalObject]) -> evaluator::Result {
    let mut map = args[0].as_map()?.clone();
    // TODO: some duplication here with types::build_map.
    for (key, value) in args[1..].iter().tuples() {
        let key = key.as_hashkey()?;
        map.insert(key.clone(), value.clone());
    }
    Ok(MalObject::wrap_map(map))
}

const DISSOC: PrimitiveFn = PrimitiveFn {
    name: "dissoc",
    fn_ptr: dissoc_,
    arity: Arity::at_least(1),
};
fn dissoc_(args: &[MalObject]) -> evaluator::Result {
    let mut map = args[0].as_map()?.clone();
    for arg in &args[1..] {
        map.remove(&MalObject::as_hashkey(arg)?);
    }
    Ok(MalObject::wrap_map(map))
}

const GET: PrimitiveFn = PrimitiveFn {
    name: "get",
    fn_ptr: get_,
    arity: Arity::exactly(2),
};
fn get_(args: &[MalObject]) -> evaluator::Result {
    if args[0].is_nil() {
        return Ok(MalObject::Nil);
    }
    let map = args[0].as_map()?;
    let key = &args[1].as_hashkey()?;
    Ok(map.get(key).unwrap_or(&MalObject::Nil).clone())
}

const CONTAINS: PrimitiveFn = PrimitiveFn {
    name: "contains?",
    fn_ptr: contains_,
    arity: Arity::exactly(2),
};
fn contains_(args: &[MalObject]) -> evaluator::Result {
    let map = args[0].as_map()?;
    let key = &args[1].as_hashkey()?;
    Ok(MalObject::Bool(map.get(key).is_some()))
}

const KEYS: PrimitiveFn = PrimitiveFn {
    name: "keys",
    fn_ptr: keys_,
    arity: Arity::exactly(1),
};
fn keys_(args: &[MalObject]) -> evaluator::Result {
    let keys = args[0]
        .as_map()?
        .keys()
        .map(HashKey::into_mal_object)
        .collect();
    Ok(MalObject::wrap_list(keys))
}

const VALS: PrimitiveFn = PrimitiveFn {
    name: "vals",
    fn_ptr: vals_,
    arity: Arity::exactly(1),
};
fn vals_(args: &[MalObject]) -> evaluator::Result {
    let vals = args[0].as_map()?.values().cloned().collect();
    Ok(MalObject::wrap_list(vals))
}

const THROW: PrimitiveFn = PrimitiveFn {
    name: "throw",
    fn_ptr: throw_,
    arity: Arity::exactly(1),
};
fn throw_(args: &[MalObject]) -> evaluator::Result {
    Err(evaluator::Error::UserException(args[0].clone()))
}

const STRING_TEST: PrimitiveFn = PrimitiveFn {
    name: "string?",
    fn_ptr: string_test,
    arity: Arity::exactly(1),
};
fn string_test(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(args[0].is_string()))
}

const NUMBER_TEST: PrimitiveFn = PrimitiveFn {
    name: "number?",
    fn_ptr: number_test,
    arity: Arity::exactly(1),
};
fn number_test(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(args[0].is_number()))
}

const FUNCTION_TEST: PrimitiveFn = PrimitiveFn {
    name: "fn?",
    fn_ptr: function_test,
    arity: Arity::exactly(1),
};
fn function_test(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(callable(&args[0]) && !args[0].is_macro()))
}

const MACRO_TEST: PrimitiveFn = PrimitiveFn {
    name: "macro?",
    fn_ptr: macro_test,
    arity: Arity::exactly(1),
};
fn macro_test(args: &[MalObject]) -> evaluator::Result {
    Ok(MalObject::Bool(args[0].is_macro()))
}

const READLINE: PrimitiveFn = PrimitiveFn {
    name: "readline",
    fn_ptr: readline_,
    arity: Arity::exactly(1),
};
fn readline_(args: &[MalObject]) -> evaluator::Result {
    let prompt = args[0].as_string()?;
    lazy_static! {
        pub static ref INTERFACE: Interface<DefaultTerminal> =
            linefeed::Interface::new("mal_user").unwrap();
    }
    INTERFACE.set_prompt(prompt)?;
    match INTERFACE.read_line() {
        Ok(ReadResult::Eof) => Ok(MalObject::Nil),
        Ok(ReadResult::Signal(_)) => Ok(MalObject::Nil),
        Ok(ReadResult::Input(i)) => Ok(MalObject::String(i)),
        Err(e) => Err(e.into()),
    }
}

const TIME_MS: PrimitiveFn = PrimitiveFn {
    name: "time-ms",
    fn_ptr: time_ms_,
    arity: Arity::exactly(0),
};
fn time_ms_(_args: &[MalObject]) -> evaluator::Result {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap(); // TODO really ought not to hide this!
    Ok(MalObject::Integer(duration.as_millis() as MalInt))
}

type Namespace = HashMap<&'static str, &'static PrimitiveFn>;
lazy_static! {
    pub static ref CORE: Namespace = {
        let mut map = Namespace::new();
        for func in [
            // Arithmetic
            SUM,
            SUB,
            MUL,
            DIV,
            // Comparisons
            GT,
            GE,
            LT,
            LE,
            // Working with strings
            PR_STR,
            STR,
            PRN,
            PRINTLN,
            READ_STRING,
            SLURP,
            // Working with lists
            CONS,
            CONCAT,
            NTH,
            FIRST,
            REST,
            APPLY,
            MAP,
            SEQ,
            CONJ,
            // Working with maps
            HASH_MAP,
            ASSOC,
            DISSOC,
            GET,
            CONTAINS,
            KEYS,
            VALS,
            // Working with atoms
            DEREF,
            RESET,
            SWAP,
            // Casting and testing
            NIL_TEST,
            TRUE_TEST,
            FALSE_TEST,
            LIST,
            LIST_TEST,
            VECTOR,
            VECTOR_TEST,
            SEQUENTIAL_TEST,
            EMPTY_TEST,
            COUNT,
            EQUAL,
            ATOM,
            ATOM_TEST,
            SYMBOL,
            SYMBOL_TEST,
            KEYWORD,
            KEYWORD_TEST,
            MAP_TEST,
            FUNCTION_TEST,
            MACRO_TEST,
            STRING_TEST,
            NUMBER_TEST,
            // Exceptions
            THROW,
            // Other
            READLINE,
            TIME_MS,
        ].iter() {
            map.insert(func.name, func);
        }
        map
    };
}
