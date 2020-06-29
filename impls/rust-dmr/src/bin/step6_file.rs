use rust_dmr_mal::{cmdline, environment, interpreter};
use std::rc::Rc;

fn main() -> std::io::Result<()> {
    let env = Rc::new(environment::Environment::default());
    environment::read_prelude(&env).expect("error during setup");
    environment::add_eval(&env);
    cmdline::run(|line| interpreter::rep(line, &env))
}
