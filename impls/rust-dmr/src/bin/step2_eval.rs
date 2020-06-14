use rust_dmr_mal::{cmdline, interpreter};

fn main() -> std::io::Result<()> {
    cmdline::run(interpreter::rep)
}
