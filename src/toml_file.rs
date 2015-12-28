use toml::Parser;
use regex::Regex;
use std::io::prelude::*;
use std::fs::File;
use std::io::Error;
use std::fs::OpenOptions;

#[derive(Debug)]
pub enum TomlError {
    parseError(&'static str),
    ioError(Error)
}

pub fn read_version(file: String) -> Option<String> {
   let file_map = Parser::new(&file).parse().unwrap();
   let package = file_map.get("package").unwrap();
   let version = package.as_table()
       .unwrap()
       .get("version");
   match version {
       Some(v) => Some(v.as_str().unwrap().into()),
       None => None
   }
}

pub fn file_with_new_version(file: String, new_version: &str) -> String {
    let re = Regex::new(r#"version\s=\s"\d+\.\d+\.\d+""#).unwrap();
    let new_version = format!("version = \"{}\"", new_version);
    re.replace(&file, &new_version[..])
}

pub fn read_from_file() -> Result<String, TomlError> {
    let cargo_file = match read_cargo_toml() {
        Ok(buffer) => buffer,
        Err(err) => return Err(TomlError::ioError(err))
    };

    match read_version(cargo_file) {
        Some(version) => Ok(version),
        None => Err(TomlError::parseError("No version field found"))
    }
}

pub fn write_new_version(new_version: String) -> Result<(), Error> {
    let cargo_toml = try!(read_cargo_toml());
    let new_cargo_toml = file_with_new_version(cargo_toml, &new_version);
    let mut handle = try!(OpenOptions::new().read(true).write(true).open("Cargo.toml"));
    handle.write_all(new_cargo_toml.as_bytes())
}

fn read_cargo_toml() -> Result<String, Error> {
    let mut handle = match File::open("Cargo.toml") {
        Ok(handle) => handle,
        Err(err) => {
            return Err(err)
        }
    };

    let mut buffer = String::new();
    match handle.read_to_string(&mut buffer) {
        Ok(_) => Ok(buffer),
        Err(err) => Err(err)
    }
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

    fn example_file_without_version() -> String {
        "[package]
    name = \"semantic-rs\"
    authors = [\"Jan Schulte <hello@unexpected-co.de>\"]
    [dependencies]
    term = \"0.2\"
    toml = \"0.1\"".to_string()
    }

    #[test]
    fn read_version_number() {
        let version_str = read_version(example_file());
        assert_eq!(version_str, Some("0.1.0".into()));
    }

    #[test]
    fn read_file_without_version_number() {
        let version_str = read_version(example_file_without_version());
        assert_eq!(version_str, None);
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
