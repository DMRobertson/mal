use regex::Regex;

#[derive(Debug)]
pub enum Token<'a> {
    SpliceUnquote,
    // Special chars
    OpenSquareBracket,
    CloseSquareBracket,
    OpenBraceBracket,
    CloseBraceBracket,
    OpenRoundBracket,
    CloseRoundBracket,
    Quote,
    Backtick,
    Tilde,
    Caret,
    AtSign,
    // Everything else
    StringLiteral(&'a str),
    Comment(&'a str),
    PlainChars(&'a str),
}

#[derive(Debug)]
pub enum TokenCreationError {
    NoFirstCharacter,
    BadTildeMatch,
    UnclosedString,
}

fn create_token(captured: &str) -> Result<Token, TokenCreationError> {
    let mut chars = captured.chars();
    let first_char = chars.next().ok_or(TokenCreationError::NoFirstCharacter)?;
    match first_char {
        // Splice unquote and special chars
        '~' => {
            if captured.len() == 1 {
                Ok(Token::Tilde)
            } else if let Some('@') = chars.next() {
                Ok(Token::SpliceUnquote)
            } else {
                Err(TokenCreationError::BadTildeMatch)
            }
        }
        '[' => Ok(Token::OpenSquareBracket),
        '{' => Ok(Token::OpenBraceBracket),
        '(' => Ok(Token::OpenRoundBracket),
        ']' => Ok(Token::CloseSquareBracket),
        '}' => Ok(Token::CloseBraceBracket),
        ')' => Ok(Token::CloseRoundBracket),
        '\'' => Ok(Token::Quote),
        '`' => Ok(Token::Backtick),
        '^' => Ok(Token::Caret),
        '@' => Ok(Token::AtSign),
        // String literal
        '"' => match chars.last() {
            Some('"') => {
                // Assuming UTf-8 encoded, first and last bytes will be ASCII ", so safe to
                // slice on bytes even if rest of the string is non ASCII.
                Ok(Token::StringLiteral(&captured[1..captured.len()]))
            }
            _ => Err(TokenCreationError::UnclosedString),
        },
        // Comment. Note that ; is ASCII so safe to slice on bytes even if the rest of the string is
        // non ASCII.
        ';' => Ok(Token::Comment(&captured[1..])),
        _ => Ok(Token::PlainChars(&captured)),
    }
}

#[derive(Debug)]
pub enum TokenizerError {
    Creation(TokenCreationError),
    NoTokens,
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, TokenizerError> {
    lazy_static! {
        static ref TOKEN_RE: Regex = Regex::new(
            r#"(?x)                          # ignore whitespace and allow comments
                [\s,]*                       # whitespace or commas, ignored
                (                            # token capture group
                    ~@                       # literal splice-unquote 
                    |[\[\]{}()'`~^@]         # single special characters
                    |"(?:                    # string literal:
                        \\.|[^\\"]           #    quotes escaped by backslashes 
                    )*\\"?                   #    possibly missing a closing quote
                    |;.*                     # comments
                    |[^\s\[\]{}('\\"`,;)]*   # zero or more plain characters
                )"#
        )
        .unwrap();
    }

    let mut captures = TOKEN_RE.captures_iter(input).peekable();
    captures.peek().ok_or(TokenizerError::NoTokens)?;
    let result = captures
        .map(|captures| captures.get(0).unwrap().as_str())
        .map(create_token)
        .collect::<Result<Vec<Token>, TokenCreationError>>()
        .map_err(|e| TokenizerError::Creation(e));
    result
}
