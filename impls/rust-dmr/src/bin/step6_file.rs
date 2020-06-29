use rust_dmr_mal::{cmdline, environment};
use std::rc::Rc;

fn main() -> Result<(), cmdline::Error> {
    log::debug!("make env");
    let env = Rc::new(environment::Environment::default());
    log::debug!("read prelude");
    environment::read_prelude(&env).expect("error during setup");
    log::debug!("allowing eval in this environment");
    environment::add_eval(&env);
    let args = std::env::args().collect();
    cmdline::launch(args, &env)
}
