use rust_dmr_mal::{cmdline, printer};

fn read(line: &str) -> &str {
    line
}

fn eval(line: &str) -> &str {
    line
}

fn print(line: &str) -> printer::Result {
    Ok(printer::Outcome::String(line.to_string()))
}

fn rep(line: &str) -> printer::Result {
    print(eval(read(&line)))
}

fn main() -> std::io::Result<()> {
    let interface = cmdline::setup()?;
    cmdline::repl(&interface, rep);
    cmdline::save_history(&interface)?;
    Ok(())
}
