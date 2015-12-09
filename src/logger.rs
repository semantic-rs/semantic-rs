use std::convert::AsRef;
extern crate term;
use std::io::prelude::*;

pub enum MessageType {
    Info,
    Success,
    Error
}

pub fn stdout<P: AsRef<str>>(message: P) {
    print_message(message, MessageType::Info);
}

pub fn success<P: AsRef<str>>(message: P) {
    print_message(message, MessageType::Success);
}

pub fn stderr<P: AsRef<str>>(message: P) {
    print_message(message, MessageType::Error);
}

fn print_message<P: AsRef<str>>(message: P, message_type: MessageType) {
    match message_type {
        MessageType::Info => {
            println!("{}", message.as_ref());
        },
        MessageType::Success => {
            let mut success_terminal = term::stdout().unwrap();
            success_terminal.fg(term::color::GREEN).unwrap();
            writeln!(success_terminal, "{}", message.as_ref()).unwrap();
            success_terminal.reset().unwrap();
        },
        MessageType::Error => {
            let mut error_terminal = term::stderr().unwrap();
            error_terminal.fg(term::color::RED).unwrap();
            writeln!(error_terminal, "{}", message.as_ref()).unwrap();
            error_terminal.reset().unwrap();
        }
    }
}
