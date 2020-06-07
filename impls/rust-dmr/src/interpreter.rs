use crate::types::MalObject;
use crate::{evaluator, printer, reader};

pub type Result = std::result::Result<MalObject, Error>;
#[derive(Debug)]
pub enum Error {
    Read(reader::Error),
    Eval(evaluator::Error),
}

#[allow(non_snake_case)]
pub fn READ(line: &str) -> Result {
    reader::read_str(line).map_err(Error::Read)
}

#[allow(non_snake_case)]
pub fn PRINT(result: &Result) -> printer::Result {
    printer::print(result)
}
