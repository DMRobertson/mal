use rust_dmr_mal::interpreter::{PRINT, READ};
use rust_dmr_mal::types::MalObject;
use rust_dmr_mal::{cmdline, environment, interpreter, printer};
use std::rc::Rc;

#[allow(non_snake_case)]
fn EVAL(result: MalObject) -> interpreter::Result {
    Ok(result)
}

fn rep(line: &str) -> printer::Result {
    PRINT(&READ(&line).and_then(EVAL))
}

fn main() -> std::io::Result<()> {
    let rep_dummy = |s: &str, _: &Rc<environment::Environment>| rep(s);
    cmdline::run(rep_dummy)
}
