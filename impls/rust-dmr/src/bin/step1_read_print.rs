use rust_dmr_mal::{cmdline, printer, reader};

fn read(line: &str) -> reader::Result {
    reader::read_str(line)
}

fn eval(result: reader::Result) -> reader::Result {
    result.map(|obj| obj)
}

fn print(result: reader::Result) -> Result<String, String> {
    log::debug!("print {:?}", result);
    result
        .map(|obj| printer::pr_str(&obj))
        .map_err(|e| format!("{:?}", e))
}

fn rep(line: &str) -> Result<String, String> {
    print(eval(read(&line)))
}

fn main() -> std::io::Result<()> {
    pretty_env_logger::init();
    let interface = cmdline::setup()?;
    cmdline::repl(&interface, rep);
    cmdline::save_history(&interface)?;
    Ok(())
}
