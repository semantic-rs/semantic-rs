#[path="../src/toml_file.rs"]
mod toml_file;
extern crate toml;
extern crate regex;

fn example_file() -> String {
    "[package]
    name = \"semantic-rs\"
    version = \"0.1.0\"
    authors = [\"Jan Schulte <hello@unexpected-co.de>\"]
    [dependencies]
    term = \"0.2\"
    toml = \"0.1\"".to_string()
}

#[test]
fn read_version_number() {
    let version_str = toml_file::read_version(example_file());
    assert_eq!(version_str, "0.1.0");
}

#[test]
fn write_new_version_number() {
    let new_toml_file = toml_file::file_with_new_version(example_file(), "0.2.0".into());
    let expected_file =
    "[package]
    name = \"semantic-rs\"
    version = \"0.2.0\"
    authors = [\"Jan Schulte <hello@unexpected-co.de>\"]
    [dependencies]
    term = \"0.2\"
    toml = \"0.1\"".to_string();
    assert_eq!(new_toml_file, expected_file);
}
