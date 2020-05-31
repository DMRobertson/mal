use crate::{environment, evaluator, reader, MalObject};

pub type Result = std::result::Result<MalObject, Error>;
#[derive(Debug)]
pub enum Error {
    Read(reader::Error),
    Eval(evaluator::Error),
}

pub fn read(line: &str) -> Result {
    reader::read_str(line).map_err(Error::Read)
}

pub fn eval(obj: &MalObject, env: &mut environment::EnvironmentStack) -> Result {
    evaluator::eval(obj, env).map_err(Error::Eval)
}
