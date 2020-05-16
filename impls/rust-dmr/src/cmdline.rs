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

pub fn repl<T: Terminal>(interface: &Interface<T>, processor: impl Fn(&str) -> &str) {
    loop {
        match interface.read_line() {
            Ok(ReadResult::Eof) => break,
            Ok(ReadResult::Signal(sig)) => {
                writeln!(interface, "Received signal {:?}", sig).ok();
            }
            Ok(ReadResult::Input(line)) => {
                interface.add_history_unique(line.clone());
                writeln!(interface, "{}", processor(&line)).ok();
            }
            Err(e) => {
                writeln!(interface, "Error: {}", e).ok();
                break;
            }
        }
    }
}
