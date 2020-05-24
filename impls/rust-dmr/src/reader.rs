use crate::tokens::{tokenize, SpecialChar, Token, TokenizerError};
use crate::types::MalObject;
use std::iter::Peekable;
use std::{fmt, slice};

type Reader<'a> = Peekable<slice::Iter<'a, Token<'a>>>;

#[derive(Debug)]
pub enum ReadError {
    TokenizerError(TokenizerError),
    NoMoreTokens,
    UnbalancedSequence,
    ReadIntError,
    ReadComment,
    UnexpectedSpecialChar, // TODO include the actual char here as payload
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Read error: {}",
            match self {
                Self::TokenizerError(e) => format!("{}", e),
                Self::NoMoreTokens =>
                    String::from("ran out of tokens while scanning for a compound type."),
                Self::UnbalancedSequence =>
                    String::from("unbalanced sequence: list or vector missing a closing bracket."),
                Self::ReadIntError => String::from("failed to parse integer."),
                Self::ReadComment => String::from("read a comment instead of object"),
                Self::UnexpectedSpecialChar =>
                    String::from("unexpected special character while parsing"),
            }
        )
    }
}

pub type Result = std::result::Result<MalObject, ReadError>;

pub fn read_str(input: &str) -> Result {
    let tokens = tokenize(input).map_err(|e| ReadError::TokenizerError(e))?;
    log::debug!("tokenize produced {:?}", tokens);
    let mut reader = tokens.iter().peekable();
    read_form(&mut reader)
}

fn read_form(reader: &mut Reader) -> Result {
    let token = reader.next().ok_or(ReadError::NoMoreTokens)?;
    log::debug!("read_form, token={:?}", token);
    use SpecialChar::*;
    match token {
        Token::SpecialChar(OpenRoundBracket) => read_list(reader),
        Token::SpecialChar(OpenSquareBracket) => read_vector(reader),
        Token::SpecialChar(CloseRoundBracket)
        | Token::SpecialChar(CloseSquareBracket)
        | Token::SpecialChar(CloseBraceBracket) => Err(ReadError::UnexpectedSpecialChar),
        Token::PlainChars(_) => read_atom(token),
        Token::StringLiteral(s) => Ok(build_string(s)),
        Token::Comment(_) => Err(ReadError::ReadComment),
        token => unimplemented!("Not implemented: {:?}", token),
    }
}

fn read_list(reader: &mut Reader) -> Result {
    read_sequence(reader, SpecialChar::CloseRoundBracket).map(MalObject::List)
}

fn read_vector(reader: &mut Reader) -> Result {
    read_sequence(reader, SpecialChar::CloseRoundBracket).map(MalObject::Vector)
}

fn read_sequence(
    reader: &mut Reader,
    closing_token: SpecialChar,
) -> std::result::Result<Vec<MalObject>, ReadError> {
    log::debug!("read_sequence, looking for {:?}", closing_token);
    let mut elements = Vec::<MalObject>::new();
    // opening token already consumed
    loop {
        log::debug!("read_sequence, token={:?}", reader.peek());
        match reader.peek() {
            Some(Token::SpecialChar(c)) if *c == closing_token => {
                reader.next();
                break;
            }
            Some(_token) => elements.push(read_form(reader)?),
            None => Err(ReadError::UnbalancedSequence)?,
        }
    }
    Ok(elements)
}

fn read_atom(token: &Token) -> Result {
    match token {
        Token::PlainChars(chars) => read_plain_chars(chars),
        Token::StringLiteral(chars) => Ok(build_string(chars)),
        token => unimplemented!("read_atom token {:?}", token),
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
