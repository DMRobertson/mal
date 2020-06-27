use rust_dmr_mal::{cmdline, environment, printer};
use std::rc::Rc;

#[allow(non_snake_case)]
fn READ(line: &str) -> &str {
    line
}

#[allow(non_snake_case)]
fn EVAL(line: &str) -> &str {
    line
}

#[allow(non_snake_case)]
fn PRINT(line: &str) -> printer::Result {
    Ok(printer::Outcome::String(line.to_string()))
}

fn rep(line: &str) -> printer::Result {
    PRINT(EVAL(READ(&line)))
}

fn main() -> std::io::Result<()> {
    let rep_dummy = |s: &str, _: &Rc<environment::Environment>| rep(s);
    cmdline::run(rep_dummy)
}
