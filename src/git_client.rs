use std::process::{Command};
use git_history;

fn parse_log_entries(git_stdout: String) -> Vec<git_history::LogEntry> {
    git_history::parse_log(git_stdout)
}

pub fn log() -> Result<Vec<git_history::LogEntry>, String> {
    let shellout = Command::new("git")
        .arg("log")
        .arg("--format=%H==SPLIT==%B==END==")
        .output()
        .unwrap();

    match shellout.status.success() {
        true => Ok(parse_log_entries(String::from_utf8_lossy(&shellout.stdout).to_string())),
        false => Err(String::from_utf8_lossy(&shellout.stderr).to_string())
    }
}
