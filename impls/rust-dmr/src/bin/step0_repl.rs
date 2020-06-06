use rust_dmr_mal::{cmdline, environment, printer};

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
    let rep_dummy = |s: &str, _: &mut environment::EnvironmentStack| rep(s);
    cmdline::run(rep_dummy)
}
