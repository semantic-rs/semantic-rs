pub struct LogEntry {
    pub title: String
}

impl LogEntry {
    pub fn new(log_string: String) -> LogEntry {
        let title = log_string.split("==SPLIT==").last()
            .unwrap()
            .trim()
            .to_string();

        LogEntry { title: title }
    }
}


pub fn parse_log(history: String) -> Vec<LogEntry> {
    history.split("==END==")
        .map(|st| st.to_string())
        .filter(|st| st.len() > 0)
        .map(|st| LogEntry::new(st))
        .collect::<Vec<_>>()
}
