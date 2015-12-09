#[path="../src/git_history.rs"]
mod git_history;

fn example_history() -> String {
    "1ef4c7432745a7d4d06cde325a89afb45fbd587a==SPLIT==Merge remote-tracking branch 'origin/master'
        ==END==
        1fda30fd9a331d18ccd7dfbcc5c20b506635e404==SPLIT==add travis-ci badge
        ==END==
        07cc47854cb30ba55652ccc7e2db591b11df8073==SPLIT==Merge pull request #41 from schultyy/add-travis-config

        Create .travis.yml==END==
        c0dc43c546a5387c546759a5c99522d46a773b32==SPLIT==Create .travis.yml==END==
        8b86a418b412a2c5b5d58ffb1c681ae93d293c6e==SPLIT==Merge pull request #40 from schultyy/use-os_type-crate

        use os_type crate==END==".into()
}

#[test]
fn convert_git_history_into_object_model() {
    let log_entries = git_history::parse_log(example_history());
    println!("{:?}", log_entries);
    assert_eq!(log_entries.len(), 5);
}

