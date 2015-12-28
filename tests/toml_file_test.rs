#[path="../src/toml_file.rs"]
mod toml_file;
extern crate toml;

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
