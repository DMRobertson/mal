use crate::environment::Environment;
use crate::evaluator::EVAL;
use crate::types::{MalList, MalObject, MalSymbol};
use crate::{interpreter, printer};
use ansi_term::Style;
use linefeed::{DefaultTerminal, Interface, ReadResult, Terminal};
use std::cmp::min;
use std::path::PathBuf;
use std::rc::Rc;

pub fn run<F>(rep: F) -> std::io::Result<()>
where
    F: Fn(&str) -> printer::Result,
{
    pretty_env_logger::init();
    let interface = setup()?;
    let processor = |line: &str| rep(line);
    repl(&interface, processor);
    save_history(&interface)?;
    Ok(())
}

fn setup() -> std::io::Result<Interface<DefaultTerminal>> {
    let interface = linefeed::Interface::new("mal")?;
    interface.set_prompt("user> ")?;
    if let Some(path) = history_path() {
        interface.load_history(path).ok();
    };
    Ok(interface)
}

fn history_path() -> Option<PathBuf> {
    match dirs::data_dir() {
        Some(mut path) => {
            path.push(".mal_history");
            Some(path)
        }
        None => None,
    }
}

fn save_history<T: Terminal>(interface: &Interface<T>) -> std::io::Result<()> {
    match history_path() {
        Some(path) => interface.save_history(path),
        None => Ok(()),
    }
}

lazy_static! {
    static ref ERROR: Style = Style::new();
    static ref WARN: Style = Style::new();
}

struct Styles {
    error: Style,
    warn: Style,
}

fn setup_colors() -> Styles {
    if atty::is(atty::Stream::Stdout) {
        Styles {
            error: Style::new().fg(ansi_term::Color::Red).bold(),
            warn: Style::new().fg(ansi_term::Color::Yellow),
        }
    } else {
        Styles {
            error: Style::new(),
            warn: Style::new(),
        }
    }
}

fn repl<T, F>(interface: &Interface<T>, mut processor: F)
where
    T: Terminal,
    F: FnMut(&str) -> printer::Result,
{
    use printer::Outcome;
    let styles = setup_colors();
    loop {
        match interface.read_line() {
            Ok(ReadResult::Eof) => break,
            Ok(ReadResult::Signal(sig)) => {
                let msg = format!("Received signal {:?}", sig);
                writeln!(interface, "{}", styles.warn.paint(msg)).ok();
            }
            Ok(ReadResult::Input(line)) => {
                if line.trim().is_empty() {
                    continue;
                }
                interface.add_history_unique(line.clone());
                match processor(&line) {
                    Ok(Outcome::String(s)) => writeln!(interface, "{}", s).ok(),
                    Ok(Outcome::Empty) => continue,
                    Err(e) => writeln!(interface, "{}", styles.error.paint(e)).ok(),
                };
            }
            Err(e) => {
                writeln!(interface, "Error: {}", e).ok();
                break;
            }
        }
    }
}

enum Mode {
    Repl,
    Batch(String),
}

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    BadArguments,
    RepError(String),
}

fn process_argv(args: &Vec<String>) -> Mode {
    log::debug!("Batch mode, args={:?}", args);
    match args.as_slice() {
        [] | [_] => Mode::Repl,
        [_program_name, file_path, ..] => Mode::Batch(file_path.clone()),
    }
}

pub fn launch(mut args: Vec<String>, env: &Rc<Environment>) -> Result<(), Error> {
    let mode = process_argv(&args);

    let script_args = args.split_off(min(args.len(), 2));
    let script_args = MalObject::wrap_list(
        script_args
            .into_iter()
            .map(|s| MalObject::String(s))
            .collect(),
    );
    env.set(MalSymbol::from("*ARGV*"), script_args);

    match mode {
        Mode::Repl => run(|line| interpreter::rep(line, &env)).map_err(Error::IO),
        Mode::Batch(path) => {
            let cmd: MalList = vec![
                MalObject::new_symbol("load-file"),
                MalObject::String(path.to_string()),
            ];
            log::debug!("Batch mode, run cmd {:?}", cmd);
            match EVAL(&MalObject::wrap_list(cmd), &env) {
                Ok(_) => Ok(()),
                Err(e) => Err(Error::RepError(e.to_string())),
            }
        }
    }
}
