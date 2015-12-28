use toml::Parser;

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
