use crate::printer;
use ansi_term::Style;
use linefeed::{DefaultTerminal, Interface, ReadResult, Terminal};
use std::path::PathBuf;

pub fn setup() -> std::io::Result<Interface<DefaultTerminal>> {
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

pub fn save_history<T: Terminal>(interface: &Interface<T>) -> std::io::Result<()> {
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

pub fn repl<T, F>(interface: &Interface<T>, mut processor: F)
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
