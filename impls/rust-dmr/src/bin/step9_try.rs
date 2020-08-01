use rust_dmr_mal::{cmdline, environment};
use std::rc::Rc;

fn main() -> Result<(), cmdline::Error> {
    let env = Rc::new(environment::Environment::default());
    environment::read_prelude(&env).expect("error reading prelude");
    environment::add_eval(&env);
    let args = std::env::args().collect();
    cmdline::launch(args, &env)
}
