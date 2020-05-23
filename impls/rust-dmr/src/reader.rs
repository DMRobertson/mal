use crate::reader::ReadError::ReadIntError;
use crate::tokens::{tokenize, Token, TokenizerError};
use crate::types::{MalList, MalObject};
use std::iter::Peekable;
use std::net::Shutdown::Read;
use std::slice;

type Reader<'a> = Peekable<slice::Iter<'a, Token<'a>>>;

pub enum ReadError {
    TokenizerError(TokenizerError),
    NoMoreTokens,
    UnclosedList,
    ReadIntError,
    ReadAtomError,
}

pub fn read_str(input: &str) -> Result<MalObject, ReadError> {
    let tokens = tokenize(input).map_err(|e| ReadError::TokenizerError(e))?;
    let mut reader = tokens.iter().peekable();
    read_form(&mut reader)
}

fn read_form(reader: &mut Reader) -> Result<MalObject, ReadError> {
    match reader.peek() {
        Some(Token::OpenRoundBracket) => read_list(reader).map(|list| MalObject::List(list)),
        Some(Token::PlainChars(_)) => read_atom(reader),
        Some(lhs) => panic!("Not implemented: {:#?}", lhs),
        None => Err(ReadError::NoMoreTokens),
    }
}

fn read_list(reader: &mut Reader) -> Result<MalList, ReadError> {
    let mut elements = MalList::new();
    loop {
        let next = reader.next();
        match next {
            Some(Token::CloseRoundBracket) => break,
            Some(token) => elements.push(read_form(reader)?),
            None => Err(ReadError::UnclosedList)?,
        }
    }
    Ok(elements)
}

fn read_atom(reader: &mut Reader) -> Result<MalObject, ReadError> {
    if let Some(Token::PlainChars(chars)) = reader.next() {
        match chars.chars().next().unwrap() {
            '+' | '-' | '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                i64::from_str_radix(chars, 10)
                    .or(Err(ReadError::ReadIntError))
                    .map(MalObject::Integer)
            }
            _ => Ok(MalObject::Symbol(String::from(*chars))),
        }
    } else {
        Err(ReadError::ReadAtomError)
    }
}
