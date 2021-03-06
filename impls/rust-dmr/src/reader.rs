use crate::strings::BuildError;
use crate::tokens;
use crate::tokens::{tokenize, Close, Token, TokenizerError};
use crate::types::{build_keyword, build_map, build_string, MalInt, MalObject, MapError};
use std::iter::Peekable;
use std::{fmt, slice};

type Reader<'a> = Peekable<slice::Iter<'a, Token<'a>>>;

#[derive(Debug)]
pub enum Error {
    TokenizerError(TokenizerError),
    NoMoreTokens,
    UnbalancedSequence,
    ReadIntError,
    ReadComment,
    UnexpectedCloseToken(tokens::Close),
    Unimplemented,
    ReadMapError(MapError),
    StringError(BuildError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;
        write!(f, "Read error: ")?;
        match self {
            TokenizerError(e) => write!(f, "{}", e),
            NoMoreTokens => write!(f, "ran out of tokens while scanning for a form."),
            UnbalancedSequence => write!(
                f,
                "unbalanced sequence: list or vector missing a closing bracket."
            ),
            ReadIntError => write!(f, "failed to parse integer."),
            ReadComment => write!(f, "read a comment instead of object"),
            UnexpectedCloseToken(c) => write!(f, "unexpected Close::{:?} token while parsing", c),
            ReadMapError(e) => write!(f, "{:?}", e),
            StringError(e) => write!(f, "error building string: {:?}", e),
            Unimplemented => write!(f, "haven't implemented this yet, but no need to panic!()"),
        }
    }
}

pub type Result = std::result::Result<MalObject, Error>;

pub fn read_str(input: &str) -> Result {
    let tokens = tokenize(input).map_err(Error::TokenizerError)?;
    log::trace!("tokenize produced {:?}", tokens);
    let mut reader = tokens.iter().peekable();
    let result = read_form(&mut reader);
    if let Ok(obj) = &result {
        log::trace!("read_form produced {}", obj);
    }
    result
}

fn read_form(reader: &mut Reader) -> Result {
    use crate::tokens::Open::*;
    use crate::tokens::UnaryOp::*;

    loop {
        let token = reader.next().ok_or(Error::NoMoreTokens)?;
        log::trace!("read_form, token={:?}", token);
        return match token {
            Token::Open(List) => read_list(reader),
            Token::Open(Vector) => read_vector(reader),
            Token::Open(Map) => read_map(reader),
            Token::Close(kind) => Err(Error::UnexpectedCloseToken(*kind)),
            Token::PlainChars(_) => read_atom(token),
            Token::StringLiteral(s) => build_string(s).map_err(Error::StringError),
            Token::Comment(_) => match &reader.peek() {
                None => Err(Error::ReadComment),
                Some(_) => continue,
            },
            Token::UnaryOp(Quote) => read_unary_operand(reader, "quote"),
            Token::UnaryOp(Quasiquote) => read_unary_operand(reader, "quasiquote"),
            Token::UnaryOp(Unquote) => read_unary_operand(reader, "unquote"),
            Token::UnaryOp(Deref) => read_unary_operand(reader, "deref"),
            Token::UnaryOp(SpliceUnquote) => read_unary_operand(reader, "splice-unquote"),
            Token::UnaryOp(WithMeta) => read_with_meta(reader),
        };
    }
}

fn read_list(reader: &mut Reader) -> Result {
    read_sequence(reader, Close::List).map(MalObject::wrap_list)
}

fn read_vector(reader: &mut Reader) -> Result {
    read_sequence(reader, Close::Vector).map(MalObject::wrap_vector)
}

fn read_map(reader: &mut Reader) -> Result {
    let entries = read_sequence(reader, Close::Map)?;
    build_map(entries).map_err(Error::ReadMapError)
}

fn read_sequence(
    reader: &mut Reader,
    closing_token: Close,
) -> std::result::Result<Vec<MalObject>, Error> {
    log::trace!("read_sequence, looking for {:?}", closing_token);
    let mut elements = Vec::<MalObject>::new();
    // opening token already consumed
    loop {
        log::trace!("read_sequence, token={:?}", reader.peek());
        match reader.peek() {
            Some(Token::Close(c)) if *c == closing_token => {
                reader.next();
                break;
            }
            Some(_token) => elements.push(read_form(reader)?),
            None => return Err(Error::UnbalancedSequence),
        }
    }
    Ok(elements)
}

fn read_atom(token: &Token) -> Result {
    match token {
        Token::PlainChars(chars) => read_plain_chars(chars),
        Token::StringLiteral(s) => build_string(s).map_err(Error::StringError),
        token => unimplemented!("read_atom token {:?}", token),
    }
}

fn read_plain_chars(chars: &str) -> Result {
    let mut iter = chars.chars();
    let first = iter.next().unwrap();
    match first {
        '+' | '-' => match iter.next() {
            Some(c) if ascii_digit(c) => read_int(chars),
            _ => Ok(MalObject::new_symbol(chars)),
        },
        c if ascii_digit(c) => read_int(chars),
        ':' => Ok(build_keyword(&chars[1..])),
        _ => match chars {
            "true" => Ok(MalObject::Bool(true)),
            "false" => Ok(MalObject::Bool(false)),
            "nil" => Ok(MalObject::Nil),
            _ => Ok(MalObject::new_symbol(chars)),
        },
    }
}

fn ascii_digit(c: char) -> bool {
    match c {
        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => true,
        _ => false,
    }
}

fn read_int(chars: &str) -> Result {
    MalInt::from_str_radix(chars, 10)
        .or(Err(Error::ReadIntError))
        .map(MalObject::Integer)
}

fn read_unary_operand(reader: &mut Reader, opname: &str) -> Result {
    let list = vec![MalObject::new_symbol(opname), read_form(reader)?];
    Ok(MalObject::wrap_list(list))
}

fn read_with_meta(reader: &mut Reader) -> Result {
    let mut list = Vec::new();
    list.push(MalObject::new_symbol("with-meta"));
    let first = read_form(reader)?;
    let second = read_form(reader)?;
    list.push(second);
    list.push(first);
    Ok(MalObject::wrap_list(list))
}
