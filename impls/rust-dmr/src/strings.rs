// I think Mal keeps things simple and only defines the escapes \n, \" and \\ in
// a string literal. Let's implement exactly that rather than inheriting rust's
// string literal behaviour.

use bimap::BiMap;
use std::str::Chars;

lazy_static! {
    static ref ESCAPES: BiMap<char, char> = {
        let mut m = BiMap::new();
        m.insert('\\', '\\');
        m.insert('"', '"');
        m.insert('n', '\n');
        m
    };
}
struct StringBuilder<'a> {
    chars: Chars<'a>,
}

impl<'a> StringBuilder<'a> {
    fn new(src: &'a str) -> Self {
        Self { chars: src.chars() }
    }
}

#[derive(Debug)]
pub enum BuildError {
    UnknownEscape(char),
    UnexpectedSingleBackslash,
}

impl Iterator for StringBuilder<'_> {
    type Item = std::result::Result<char, BuildError>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.chars.next()? {
            '\\' => match self.chars.next() {
                None => Err(BuildError::UnexpectedSingleBackslash),
                Some(c) => ESCAPES
                    .get_by_left(&c)
                    .copied()
                    .ok_or(BuildError::UnknownEscape(c)),
            },
            c => Ok(c),
        };
        Some(result)
    }
}

pub(crate) fn build_string(src: &str) -> Result<String, BuildError> {
    StringBuilder::new(src).collect()
}

struct StringPrinter<'a> {
    chars: Chars<'a>,
}

impl<'a> StringPrinter<'a> {
    fn new(src: &'a str) -> Self {
        Self { chars: src.chars() }
    }
}

impl Iterator for StringPrinter<'_> {
    type Item = (char, Option<char>);

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.chars.next()?;
        let charseq = match ESCAPES.get_by_right(&next) {
            Some(&l) => ('\\', Some(l)),
            None => (next, None),
        };
        Some(charseq)
    }
}

pub(crate) fn string_repr(src: &str) -> String {
    let mut output = String::new();
    output.push('"');
    for (char1, char2) in StringPrinter::new(src) {
        output.push(char1);
        if let Some(char2) = char2 {
            output.push(char2)
        };
    }
    output.push('"');
    output
}

// TODO should make baby tests to ensure that read_str and print_str_repr are
// mutually inverse but let's just rely on the mal test suite for now
