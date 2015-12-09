pub struct LogEntry {
    pub title: String,
    pub revision: String
}

impl LogEntry {
    pub fn new(log_string: String) -> LogEntry {
        let mut splitted = log_string.split("==SPLIT==");
        let revision = splitted.next()
            .unwrap()
            .trim()
            .to_string();

        let title = splitted.next()
            .unwrap()
            .trim()
            .to_string();

        LogEntry { revision: revision, title: title }
    }
}


pub fn parse_log(history: String) -> Vec<LogEntry> {
    history.split("==END==")
        .map(|st| st.to_string())
        .filter(|st| st.trim().len() > 0)
        .map(|st| LogEntry::new(st))
        .collect::<Vec<_>>()
}
