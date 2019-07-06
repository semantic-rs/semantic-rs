use std::convert::AsRef;
extern crate term;
use std::io::prelude::*;

pub enum MessageType {
    Info,
    Warn,
    Error,
}

pub fn stdout<P: AsRef<str>>(message: P) {
    print_message(message, MessageType::Info);
}

pub fn warn<P: AsRef<str>>(message: P) {
    print_message(message, MessageType::Warn);
}

pub fn stderr<P: AsRef<str>>(message: P) {
    print_message(message, MessageType::Error);
}

fn print_message<P: AsRef<str>>(message: P, message_type: MessageType) {
    match message_type {
        MessageType::Info => {
            println!("{}", message.as_ref());
        }
        MessageType::Warn => {
            let mut warn_terminal = term::stdout().unwrap();
            warn_terminal.fg(term::color::YELLOW).unwrap();
            writeln!(warn_terminal, "{}", message.as_ref()).unwrap();
            warn_terminal.reset().unwrap();
            warn_terminal.flush().unwrap();
        }
        MessageType::Error => {
            let mut error_terminal = term::stderr().unwrap();
            error_terminal.fg(term::color::RED).unwrap();
            writeln!(error_terminal, "{}", message.as_ref()).unwrap();
            error_terminal.reset().unwrap();
            error_terminal.flush().unwrap();
        }
    }
}
