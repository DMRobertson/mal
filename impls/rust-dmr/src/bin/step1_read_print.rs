use rust_dmr_mal::{cmdline, printer, reader, MalObject};

fn read(line: &str) -> MalObject {
    reader::read_str(line).ok().unwrap()
}

fn eval(obj: MalObject) -> MalObject {
    obj
}

fn print(obj: &MalObject) -> String {
    printer::pr_str(obj)
}

fn rep(line: &str) -> String {
    print(&eval(read(&line)))
}

fn main() -> std::io::Result<()> {
    pretty_env_logger::init();
    let interface = cmdline::setup()?;
    cmdline::repl(&interface, rep);
    cmdline::save_history(&interface)?;
    Ok(())
}
