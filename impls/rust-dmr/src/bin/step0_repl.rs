use std::io::Write;

fn read(line: &str) -> &str {
    line
}

fn eval(line: &str) -> &str {
    line
}

fn print(line: &str) -> &str {
    line
}

fn rep(line: &str) -> &str {
    print(eval(read(&line)))
}

fn main() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut line = String::new();
    loop {
        const PROMPT: &[u8] = b"user> ";
        stdout.write(PROMPT)?;
        stdout.flush()?;
        let bytes_read = stdin.read_line(&mut line)?;
        if bytes_read == 0 {
            break;
        }
        stdout.write(rep(&line).as_bytes())?;
        line.clear();
    }
    Ok(())
}
