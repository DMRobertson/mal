use rust_dmr_mal::{cmdline, environment, interpreter};
use std::rc::Rc;

fn main() -> std::io::Result<()> {
    let env = Rc::new(environment::Environment::default());
    // interpreter::rep("(def! not (fn* (a) (if a false true)))", &env).expect("Error during setup");
    cmdline::run(|line| interpreter::rep(line, &env))
}
