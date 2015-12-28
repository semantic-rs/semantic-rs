use toml::Parser;
use regex::Regex;

pub fn read_version(file: String) -> String {
   let file_map = Parser::new(&file).parse().unwrap();
   println!("{:?}", file_map);
   let package = file_map.get("package").unwrap();
   package.as_table()
       .unwrap()
       .get("version").unwrap()
       .as_str()
       .unwrap()
       .into()
}

pub fn file_with_new_version(file: String, new_version: &str) -> String {
    let re = Regex::new(r#"version\s=\s"\d+\.\d+\.\d+""#).unwrap();
    let new_version = format!("version = \"{}\"", new_version);
    re.replace(&file, &new_version[..])
}

#[cfg(test)]
mod tests {
    extern crate toml;
    extern crate regex;
    use super::*;

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
        let version_str = read_version(example_file());
        assert_eq!(version_str, "0.1.0");
    }

    #[test]
    fn write_new_version_number() {
        let new_toml_file = file_with_new_version(example_file(), "0.2.0".into());
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
}
