use rust_dmr_mal::interpreter::{PRINT, READ};
use rust_dmr_mal::types::MalObject;
use rust_dmr_mal::{cmdline, interpreter, printer};

#[allow(non_snake_case)]
fn EVAL(result: MalObject) -> interpreter::Result {
    Ok(result)
}

fn rep(line: &str) -> printer::Result {
    PRINT(&READ(&line).and_then(EVAL))
}

fn main() -> std::io::Result<()> {
    cmdline::run(rep)
}
