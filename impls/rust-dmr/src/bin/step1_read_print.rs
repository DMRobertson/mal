use rust_dmr_mal::{cmdline, interpreter, printer, reader};

fn read(line: &str) -> reader::Result {
    reader::read_str(line)
}

fn eval(result: reader::Result) -> reader::Result {
    result.map(|obj| obj)
}

fn rep(line: &str) -> printer::Result {
    let result = eval(read(&line));
    printer::print(&result.map_err(interpreter::Error::Read))
}

fn main() -> std::io::Result<()> {
    pretty_env_logger::init();
    let interface = cmdline::setup()?;
    cmdline::repl(&interface, rep);
    cmdline::save_history(&interface)?;
    Ok(())
}
