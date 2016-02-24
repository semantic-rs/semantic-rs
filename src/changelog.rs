use super::Config;
use std::io::BufWriter;
use clog::Clog;
use clog::fmt::MarkdownWriter;

pub fn write(config: &Config) -> Result<(), String> {
    let mut clog = try!(Clog::with_dir(&config.repository_path).map_err(|_| "Clog failed".to_owned()));

    let mut clog_file = config.repository_path.clone();
    clog_file.push("Changelog.md");

    // TODO: Make this configurable? Rely on clog's own configuration?
    clog.changelog(clog_file.to_str().unwrap())
        .from(format!("v{}", config.current_version))
        .version(format!("v{}", config.new_version));

    clog.write_changelog().map_err(|_| "Failed to write Changelog.md".to_owned())
}

pub fn generate(config: &Config) -> Result<String, String> {
    let mut clog = try!(Clog::with_dir(&config.repository_path).map_err(|_| "Clog failed".to_owned()));

    clog
        .from(format!("v{}", config.current_version))
        .version(format!("v{}", config.new_version));

    let mut out_buf = BufWriter::new(Vec::new());

    {
        let mut writer = MarkdownWriter::new(&mut out_buf);
        try!(clog.write_changelog_with(&mut writer)
             .map_err(|_| "Genearting changelog failed"))
    }

    let out_buf = out_buf.into_inner().unwrap();
    let changelog = String::from_utf8(out_buf).unwrap();

    match changelog.find('\n') {
        Some(newline_offset) => Ok(changelog[newline_offset+1..].into()),
        None => Ok(changelog)
    }
}
