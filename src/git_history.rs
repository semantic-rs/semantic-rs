pub fn parse_log(history: String) -> Vec<String> {
    history.split("==END==")
        .map(|st| st.to_string())
        .filter(|st| st.len() > 0)
        .collect::<Vec<_>>()
}
