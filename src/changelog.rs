use std::io::BufWriter;
use clog::Clog;
use clog::fmt::MarkdownWriter;

pub fn write(repository_path: &str, old_version: &str, new_version: &str) -> Result<(), String> {
    let mut clog = try!(Clog::with_dir(repository_path).map_err(|_| "Clog failed".to_owned()));

    // TODO: Make this configurable? Rely on clog's own configuration?
    clog.changelog("Changelog.md")
        .from(format!("v{}", old_version))
        .version(format!("v{}", new_version));

    clog.write_changelog().map_err(|_| "Failed to write Changelog.md".to_owned())
}

pub fn generate(repository_path: &str, old_version: &str, new_version: &str) -> Result<String, String> {
    let mut clog = try!(Clog::with_dir(repository_path).map_err(|_| "Clog failed".to_owned()));

    clog
        .from(format!("v{}", old_version))
        .version(format!("v{}", new_version));

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
