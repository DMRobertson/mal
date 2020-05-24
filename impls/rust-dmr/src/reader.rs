use crate::tokens::{tokenize, Token, TokenizerError};
use crate::types::{MalList, MalObject};
use std::iter::Peekable;
use std::slice;

type Reader<'a> = Peekable<slice::Iter<'a, Token<'a>>>;

#[derive(Debug)]
pub enum ReadError {
    TokenizerError(TokenizerError),
    NoMoreTokens,
    UnbalancedList,
    ReadIntError,
}

pub type Result = std::result::Result<MalObject, ReadError>;

pub fn read_str(input: &str) -> Result {
    let tokens = tokenize(input).map_err(|e| ReadError::TokenizerError(e))?;
    log::debug!("tokenize produced {:?}", tokens);
    let mut reader = tokens.iter().peekable();
    read_form(&mut reader)
}

fn read_form(reader: &mut Reader) -> Result {
    match reader.peek() {
        Some(Token::OpenRoundBracket) => read_list(reader),
        Some(Token::PlainChars(_)) => read_atom(reader),
        Some(Token::StringLiteral(s)) => Ok(build_string(s)),
        Some(token) => unimplemented!("Not implemented: {:?}", token),
        None => Err(ReadError::NoMoreTokens),
    }
}

fn read_list(reader: &mut Reader) -> Result {
    let mut elements = MalList::new();
    // consume Token::OpenRoundBracket
    reader.next();
    loop {
        match reader.peek() {
            Some(Token::CloseRoundBracket) => {
                reader.next();
                break;
            }
            Some(_token) => elements.push(read_form(reader)?),
            None => Err(ReadError::UnbalancedList)?,
        }
    }
    Ok(MalObject::List(elements))
}

fn read_atom(reader: &mut Reader) -> Result {
    match reader.next() {
        Some(Token::PlainChars(chars)) => read_plain_chars(chars),
        Some(Token::StringLiteral(chars)) => Ok(build_string(chars)),
        Some(token) => unimplemented!("read_atom token {:?}", token),
        None => Err(ReadError::NoMoreTokens),
    }
}

fn read_plain_chars(chars: &str) -> Result {
    let mut iter = chars.chars();
    let first = iter.next().unwrap();
    match first {
        '+' | '-' => match iter.next() {
            Some(c) if ascii_digit(c) => read_int(chars),
            _ => Ok(build_symbol(chars)),
        },
        c if ascii_digit(c) => read_int(chars),
        _ => Ok(build_symbol(chars)),
    }
}

fn ascii_digit(c: char) -> bool {
    match c {
        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => true,
        _ => false,
    }
}

fn read_int(chars: &str) -> Result {
    i64::from_str_radix(chars, 10)
        .or(Err(ReadError::ReadIntError))
        .map(MalObject::Integer)
}

fn build_symbol(chars: &str) -> MalObject {
    MalObject::Symbol(String::from(chars))
}

fn build_string(chars: &str) -> MalObject {
    MalObject::String(String::from(chars))
}
