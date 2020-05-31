use rust_dmr_mal::{cmdline, environment, interpreter, printer};

fn rep(line: &str, env: &mut environment::EnvironmentStack) -> printer::Result {
    let result = interpreter::read(&line).and_then(|obj| interpreter::eval(&obj, env));
    printer::print(&result)
}

fn main() -> std::io::Result<()> {
    pretty_env_logger::init();
    let interface = cmdline::setup()?;
    let mut envs = environment::EnvironmentStack::default();
    let processor = |line: &str| rep(line, &mut envs);
    cmdline::repl(&interface, processor);
    cmdline::save_history(&interface)?;
    Ok(())
}
