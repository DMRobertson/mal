use crate::evaluator;
use crate::types::{Arity, MalInt, MalObject, PrimitiveFn};
use std::collections::HashMap;
use std::convert::TryFrom;

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

fn mul_(args: &[MalObject]) -> evaluator::Result {
    let value = grab_ints(args)?
        .iter()
        .fold(1 as MalInt, |acc, &x| acc.wrapping_mul(x));
    Ok(MalObject::Integer(value))
}

fn sub_(args: &[MalObject]) -> evaluator::Result {
    match grab_ints(args)?.as_slice() {
        [x, y] => Ok(MalObject::Integer(x.wrapping_sub(*y))),
        _ => panic!(),
    }
}

fn div_(args: &[MalObject]) -> evaluator::Result {
    match grab_ints(args)?.as_slice() {
        [_, 0] => Err(evaluator::Error::DivideByZero),
        [x, y] => Ok(MalObject::Integer(x.wrapping_div(*y))),
        _ => panic!(),
    }
}

const SUM: PrimitiveFn = PrimitiveFn {
    name: "+",
    fn_ptr: sum_,
    arity: Arity::BoundedBelow(0..),
};
const SUB: PrimitiveFn = PrimitiveFn {
    name: "-",
    fn_ptr: sub_,
    arity: Arity::exactly(2),
};
const MUL: PrimitiveFn = PrimitiveFn {
    name: "*",
    fn_ptr: mul_,
    arity: Arity::BoundedBelow(0..),
};
const DIV: PrimitiveFn = PrimitiveFn {
    name: "/",
    fn_ptr: div_,
    arity: Arity::exactly(2),
};

type Namespace = HashMap<&'static str, &'static PrimitiveFn>;
lazy_static! {
    pub static ref CORE: Namespace = {
        let mut map = Namespace::new();
        for func in &[SUM, SUB, MUL, DIV] {
            map.insert(func.name, func);
        }
        map
    };
}
