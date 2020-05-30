use crate::strings::print_string_repr;
use regex::Regex;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Close {
    Vector,
    Map,
    List,
}

#[derive(Debug)]
pub enum Open {
    Vector,
    Map,
    List,
}

#[derive(Debug)]
pub enum UnaryOp {
    Quote,
    Quasiquote,
    Unquote,
    WithMeta,
    Deref,
    SpliceUnquote,
}

pub enum Token<'a> {
    Open(Open),
    Close(Close),
    UnaryOp(UnaryOp),
    StringLiteral(&'a str),
    Comment(&'a str),
    PlainChars(&'a str),
}

impl<'a> fmt::Debug for Token<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::StringLiteral(s) => write!(f, "StringLiteral({})", print_string_repr(s)),
            _ => write!(f, "{:?}", self),
        }
    }
}

#[derive(Debug)]
pub enum TokenizerError {
    NoFirstCharacter,
    BadTildeMatch,
    UnbalancedString,
    NoCapture(String),
}

impl fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tokenizer failed: ")?;
        match self {
            TokenizerError::NoFirstCharacter => write!(f, "no characters to parse token from"),
            TokenizerError::BadTildeMatch => write!(f, "bad tilde match"),
            TokenizerError::UnbalancedString => write!(f, "unbalanced string literal"),
            TokenizerError::NoCapture(_) => write!(f, "token regex did not capture a token"),
        }
    }
}

fn create_token(captured: &str) -> Result<Token, TokenizerError> {
    use UnaryOp::*;

    let bytes = captured.as_bytes();
    let first_char = bytes.first().ok_or(TokenizerError::NoFirstCharacter)?;
    match first_char {
        // Splice unquote and special chars
        b'~' => {
            if bytes.len() == 1 {
                Ok(Token::UnaryOp(Unquote))
            } else if let Some(b'@') = bytes.get(1) {
                Ok(Token::UnaryOp(SpliceUnquote))
            } else {
                Err(TokenizerError::BadTildeMatch)
            }
        }
        b'[' => Ok(Token::Open(Open::Vector)),
        b'{' => Ok(Token::Open(Open::Map)),
        b'(' => Ok(Token::Open(Open::List)),
        b']' => Ok(Token::Close(Close::Vector)),
        b'}' => Ok(Token::Close(Close::Map)),
        b')' => Ok(Token::Close(Close::List)),
        b'\'' => Ok(Token::UnaryOp(Quote)),
        b'`' => Ok(Token::UnaryOp(Quasiquote)),
        b'^' => Ok(Token::UnaryOp(WithMeta)),
        b'@' => Ok(Token::UnaryOp(Deref)),
        // String literal
        b'"' => tokenize_string_literal(bytes),
        // Comment. Note that ; is ASCII so safe to slice on bytes even if the rest of the string is
        // non ASCII.
        b';' => Ok(Token::Comment(&captured[1..])),
        _ => Ok(Token::PlainChars(&captured)),
    }
}

fn tokenize_string_literal(bytes: &[u8]) -> Result<Token, TokenizerError> {
    if bytes.len() == 1 || bytes[bytes.len() - 1] != b'"' {
        return Err(TokenizerError::UnbalancedString);
    }

    let trailing_backslashes = bytes
        .iter()
        .rev()
        .skip(1)
        .take_while(|&&byte| byte == b'\\')
        .count();
    if trailing_backslashes % 2 == 1 {
        return Err(TokenizerError::UnbalancedString);
    }

    Ok(Token::StringLiteral(
        std::str::from_utf8(&bytes[1..bytes.len() - 1]).unwrap(),
    ))
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, TokenizerError> {
    lazy_static! {
        static ref TOKEN_RE: Regex = Regex::new(
            r#"(?x)                          # ignore whitespace in this patern & allow comments
                [\s,]*                       # whitespace or commas, ignored
                (                            # token capture group
                    ~@                       # literal splice-unquote 
                    |[\[\]{}()'`~^@]         # single special characters
                    |"(?:                    # string literal. its contents, not captured, include:
                        \\.                  #    escapes
                        |[^\\"]              #    anything which isn't a backslash or a quote 
                      )*
                      "?                     #    possibly missing a closing quote
                    |;.*                     # comments
                    |[^\s\[\]{}('\\"`,;)]*   # zero or more plain characters
                )
                [\s,]*                       # whitespace or commas, ignored
            "#
        )
        .unwrap();
    }
    let mut input = input;
    let mut tokens = Vec::new();
    while input.len() > 0 {
        let caps = TOKEN_RE
            .captures(input)
            .ok_or(TokenizerError::NoCapture(String::from(input)))?;
        let token = create_token(caps.get(1).unwrap().as_str())?;
        tokens.push(token);
        input = &input[caps.get(0).unwrap().end()..];
    }
    Ok(tokens)
}
