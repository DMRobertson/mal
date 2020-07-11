extern crate derive_more;
use crate::environment::Environment;
use crate::strings::BuildError;
use crate::tokens::StringLiteral;
use crate::{evaluator, strings};
use derive_more::{Deref, DerefMut};
use itertools::Itertools;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;

use std::fmt::Formatter;
use std::ops::{RangeFrom, RangeInclusive};
use std::rc::Rc;
use std::{fmt, rc};

#[derive(Deref, DerefMut, Debug)]
pub struct MalList(pub Vec<MalObject>);
#[derive(Deref, DerefMut, Debug)]
pub struct MalVector(pub Vec<MalObject>);

#[derive(Deref, DerefMut, Debug)]
pub struct MalMap(pub HashMap<HashKey, MalObject>);
pub type MalInt = isize;

#[derive(Deref, Debug, PartialEq, Eq, Hash, Clone)]
pub struct MalSymbol(pub String);

impl AsRef<str> for MalSymbol {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub enum Arity {
    Between(RangeInclusive<usize>),
    AtLeast(RangeFrom<usize>),
}

#[derive(Debug)]
pub struct BadArgCount {
    name: &'static str,
    expected: Arity,
    got: usize,
}

impl fmt::Display for BadArgCount {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "When evaluating {} expected {} arguments, but received {} arguments",
            self.name, self.expected, self.got
        )
    }
}

impl Arity {
    pub(crate) const fn exactly(n: usize) -> Self {
        Self::Between(n..=n)
    }

    pub(crate) const fn at_least(n: usize) -> Self {
        Self::AtLeast(n..)
    }

    pub(crate) fn contains(&self, n: usize) -> bool {
        match self {
            Self::Between(range) => range.contains(&n),
            Self::AtLeast(range) => range.contains(&n),
        }
    }

    pub(crate) fn validate_for(&self, n: usize, name: &'static str) -> Result<(), BadArgCount> {
        match self.contains(n) {
            true => Ok(()),
            false => Err(BadArgCount {
                name,
                expected: self.clone(),
                got: n,
            }),
        }
    }
}

impl fmt::Display for Arity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arity::Between(r) => {
                if r.start() == r.end() {
                    write!(f, "exactly {}", r.start())
                } else {
                    write!(f, "from {} to {}", r.start(), r.end())
                }
            }
            Arity::AtLeast(r) => write!(f, "At least {}", r.start),
        }
    }
}

pub struct PrimitiveFn {
    pub name: &'static str,
    pub arity: Arity,
    pub fn_ptr: fn(&[MalObject]) -> evaluator::Result,
}

impl fmt::Debug for PrimitiveFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "primitive function #<{}>", self.name)
    }
}

#[derive(Clone)]
pub struct PrimitiveEval {
    pub env: rc::Weak<Environment>,
}

impl fmt::Debug for PrimitiveEval {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "PrimitiveEval")
    }
}

#[derive(Clone, Debug)]
pub struct ClosureParameters {
    pub positional: Vec<MalSymbol>,
    pub others: Option<MalSymbol>,
}

impl fmt::Display for ClosureParameters {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.positional.iter().join(" "))?;
        if let Some(rest) = &self.others {
            write!(f, " & {}", rest)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum BadClosureParameters {
    TooManyAmpersands(usize),
    TooShortForAmpersand,
    AmpersandPositionNotPenultimate,
}

impl ClosureParameters {
    pub fn new(mut symbols: Vec<MalSymbol>) -> Result<Self, BadClosureParameters> {
        let is_ampersand = |s: &&MalSymbol| ***s == "&";
        let ampersand_count = symbols.iter().filter(is_ampersand).count();

        match ampersand_count {
            0 => Ok(ClosureParameters {
                positional: symbols,
                others: None,
            }),
            1 => {
                if symbols.len() < 2 {
                    return Err(BadClosureParameters::TooShortForAmpersand);
                }
                let penultimate = symbols.get(symbols.len() - 2).unwrap();
                match is_ampersand(&penultimate) {
                    false => Err(BadClosureParameters::AmpersandPositionNotPenultimate),
                    true => {
                        let variadic_name = symbols.pop().unwrap();
                        let _ampersand = symbols.pop();
                        Ok(ClosureParameters {
                            positional: symbols,
                            others: Some(variadic_name),
                        })
                    }
                }
            }
            _ => Err(BadClosureParameters::TooManyAmpersands(ampersand_count)),
        }
    }

    pub fn arity(&self) -> Arity {
        match self.others {
            None => Arity::exactly(self.positional.len()),
            Some(_) => Arity::at_least(self.positional.len()),
        }
    }
}

#[derive(Clone)]
pub struct Closure {
    pub parameters: ClosureParameters,
    pub body: MalObject,
    pub parent: Rc<Environment>,
    pub is_macro: bool,
}

impl fmt::Debug for Closure {
    // Not derived because we want to skip the parent: the parent may well contain this Closure!
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Closure{{parameters: {:?}, body: {:?}, is_macro: {:?}}}",
            self.parameters, self.body, self.is_macro
        )
    }
}

#[derive(Debug, Clone)]
pub struct Atom {
    payload: Rc<RefCell<MalObject>>,
}

impl Atom {
    pub(crate) fn new(obj: &MalObject) -> Self {
        Self {
            payload: Rc::new(RefCell::new(obj.clone())),
        }
    }

    pub(crate) fn borrow_payload(&self) -> Ref<MalObject> {
        self.payload.borrow()
    }

    pub(crate) fn clone_payload(&self) -> MalObject {
        self.payload.borrow().clone()
    }

    pub(crate) fn replace(&self, obj: &MalObject) {
        self.payload.replace(obj.clone());
    }
}

#[derive(Debug, Clone)]
pub enum MalObject {
    Nil,
    Integer(MalInt),
    Bool(bool),
    String(String),
    Symbol(MalSymbol),
    Keyword(String),
    List(Rc<MalList>),
    Vector(Rc<MalVector>),
    Map(Rc<MalMap>),
    Primitive(&'static PrimitiveFn),
    Closure(Rc<Closure>),
    Eval(PrimitiveEval),
    Atom(Atom),
}

pub(crate) fn truthy(obj: &MalObject) -> bool {
    use MalObject::*;
    match obj {
        List(_) | Vector(_) | Map(_) | Integer(_) | Symbol(_) | String(_) | Keyword(_)
        | Primitive(_) | Closure(_) | Eval(_) | Atom(_) => true,
        Bool(t) => *t,
        Nil => false,
    }
}

pub(crate) fn callable(obj: &MalObject) -> bool {
    use MalObject::*;
    match obj {
        Primitive(_) | Closure(_) | Eval(_) => true,
        Nil => false,
        Integer(_) => false,
        Bool(_) => false,
        String(_) => false,
        Symbol(_) => false,
        Keyword(_) => false,
        List(_) => false,
        Vector(_) => false,
        Map(_) => false,
        Atom(_) => false,
    }
}

#[derive(Debug)]
pub enum TypeMismatch {
    NotAnInt,
    NotAList,
    NotASequence,
    NotASymbol,
    NotAString,
    NotAnAtom,
    NotCallable,
    NotAClosure,
}

impl MalObject {
    pub(crate) fn as_int(&self) -> Result<MalInt, TypeMismatch> {
        match self {
            MalObject::Integer(x) => Ok(*x),
            _ => Err(TypeMismatch::NotAnInt),
        }
    }

    pub(crate) fn as_list(&self) -> Result<&MalList, TypeMismatch> {
        match self {
            MalObject::List(x) => Ok(x),
            _ => Err(TypeMismatch::NotAList),
        }
    }

    pub(crate) fn as_seq(&self) -> Result<&[MalObject], TypeMismatch> {
        match self {
            MalObject::List(x) => Ok(x),
            MalObject::Vector(x) => Ok(x),
            _ => Err(TypeMismatch::NotASequence),
        }
    }

    pub(crate) fn as_symbol(&self) -> Result<&MalSymbol, TypeMismatch> {
        match self {
            MalObject::Symbol(s) => Ok(s),
            _ => Err(TypeMismatch::NotASymbol),
        }
    }

    pub(crate) fn as_closure(&self) -> Result<&Closure, TypeMismatch> {
        match self {
            MalObject::Closure(c) => Ok(c),
            _ => Err(TypeMismatch::NotAClosure),
        }
    }

    pub(crate) fn is_nil(&self) -> bool {
        match self {
            MalObject::Nil => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
// TODO Copying the string here. Is there a better way?
pub enum HashKey {
    String(String),
    Keyword(String),
}

#[derive(Debug)]
pub enum MapError {
    MissingValue,
    UnhashableKey, // TODO include the key that wasn't hashable, or at least its position
}

pub(crate) fn build_map(entries: Vec<MalObject>) -> Result<MalObject, MapError> {
    if entries.len() % 2 == 1 {
        return Err(MapError::MissingValue);
    }
    let mut map = MalMap(HashMap::new());
    for (key, value) in entries.into_iter().tuples() {
        let key = match key {
            MalObject::String(s) => Ok(HashKey::String(s)),
            MalObject::Keyword(s) => Ok(HashKey::Keyword(s)),
            _ => Err(MapError::UnhashableKey),
        }?;
        map.insert(key, value);
        // TODO detect duplicate keys?
    }
    Ok(MalObject::Map(Rc::new(map)))
}

pub(crate) fn build_keyword(chars: &str) -> MalObject {
    MalObject::Keyword(String::from(chars))
}

pub(crate) fn build_string(src: &StringLiteral) -> Result<MalObject, BuildError> {
    strings::build_string(src.payload).map(MalObject::String)
}

impl MalObject {
    pub(crate) fn new_list() -> Self {
        Self::List(Rc::new(MalList(Vec::new())))
    }
    pub(crate) fn wrap_list(elements: Vec<MalObject>) -> Self {
        Self::List(Rc::new(MalList(elements)))
    }
    pub(crate) fn wrap_vector(elements: Vec<MalObject>) -> Self {
        Self::Vector(Rc::new(MalVector(elements)))
    }
    pub(crate) fn new_symbol(name: &str) -> Self {
        Self::Symbol(MalSymbol(name.into()))
    }
}

impl PartialEq for MalObject {
    fn eq(&self, other: &Self) -> bool {
        use MalObject::*;
        if let (Some(x), Some(y)) = (self.as_seq().ok(), other.as_seq().ok()) {
            return equal_sequences(x, &y);
        }
        match [self, other] {
            [Integer(x), Integer(y)] => x == y,
            [Bool(x), Bool(y)] => x == y,
            [String(x), String(y)] => x == y,
            [Keyword(x), Keyword(y)] => x == y,
            [Symbol(x), Symbol(y)] => x == y,
            // TODO: no comparison for maps!?
            [Nil, Nil] => true,
            [_, _] => false,
        }
    }
}

// TODO Something very wrong here---shouldn't be cloning. I think the
// PrimitiveFns should be taking their args as refs! But let's get it working
// first.
// Update: Think this is fine since MalObject should be cheap to clone?
fn equal_sequences(xs: &[MalObject], ys: &[MalObject]) -> bool {
    xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| x == y)
}

impl Eq for MalObject {}
