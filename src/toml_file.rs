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
