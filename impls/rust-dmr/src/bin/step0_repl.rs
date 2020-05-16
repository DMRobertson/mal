use rust_dmr_mal::cmdline;

fn read(line: &str) -> &str {
    line
}

fn eval(line: &str) -> &str {
    line
}

fn print(line: &str) -> &str {
    line
}

fn rep(line: &str) -> &str {
    print(eval(read(&line)))
}

fn main() -> std::io::Result<()> {
    let interface = cmdline::setup()?;
    cmdline::repl(&interface, rep);
    cmdline::save_history(&interface)?;
    Ok(())
}
