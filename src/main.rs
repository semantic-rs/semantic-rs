mod git_history;
mod git_client;
mod logger;
mod toml_file;
extern crate toml;
extern crate regex;

fn print_log(log_entries: Vec<git_history::LogEntry>) {
    for entry in log_entries {
        logger::stdout(entry.revision);
        logger::stdout(entry.title);
    }
}

fn main() {
    println!("semantic.rs 🚀");

    logger::stdout("Fetched git history");
    match git_client::log() {
        Ok(log) => print_log(log),
        Err(err) => {
            logger::stderr("fatal: Failed to retrieve git history");
            logger::stderr(err);
        }
    }
}
