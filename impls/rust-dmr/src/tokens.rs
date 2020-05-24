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
pub enum TokenizerError {
    NoFirstCharacter,
    BadTildeMatch,
    UnclosedString,
    NoCapture(String),
}

fn create_token(captured: &str) -> Result<Token, TokenizerError> {
    let mut chars = captured.chars();
    let first_char = chars.next().ok_or(TokenizerError::NoFirstCharacter)?;
    match first_char {
        // Splice unquote and special chars
        '~' => {
            if captured.len() == 1 {
                Ok(Token::Tilde)
            } else if let Some('@') = chars.next() {
                Ok(Token::SpliceUnquote)
            } else {
                Err(TokenizerError::BadTildeMatch)
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
                Ok(Token::StringLiteral(&captured[1..captured.len() - 1]))
            }
            _ => Err(TokenizerError::UnclosedString),
        },
        // Comment. Note that ; is ASCII so safe to slice on bytes even if the rest of the string is
        // non ASCII.
        ';' => Ok(Token::Comment(&captured[1..])),
        _ => Ok(Token::PlainChars(&captured)),
    }
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
