use std::io::BufWriter;
use clog::Clog;
use clog::fmt::MarkdownWriter;
use std::path::PathBuf;
use std::io::prelude::*;
use std::fs::OpenOptions;
use std::io::Error;

pub fn write(repository_path: &str, old_version: &str, new_version: &str) -> Result<(), String> {
    let mut clog = try!(Clog::with_dir(repository_path).map_err(|_| "Clog failed".to_owned()));

    let mut clog_file = PathBuf::from(repository_path);
    clog_file.push("Changelog.md");

    // TODO: Make this configurable? Rely on clog's own configuration?
    clog.changelog(clog_file.to_str().unwrap())
        .from(format!("v{}", old_version))
        .version(format!("v{}", new_version));

    clog.write_changelog().map_err(|_| "Failed to write Changelog.md".to_owned())
}

pub fn write_custom(repository_path: &str, new_version: &str, changelog_text: &str) -> Result<(), Error> {
    let mut changelog_path = PathBuf::from(repository_path);
    changelog_path.push("Changelog.md");
    let mut file = match OpenOptions::new().create(true).write(true).open(changelog_path) {
        Ok(f) => f,
        Err(err) => return Err(err)
    };
    try!(file.write(format!("## {}", new_version).as_bytes()));
    match file.write(changelog_text.as_bytes()) {
        Ok(_) => Ok(()),
        Err(err) => Err(err)
    }
}

pub fn has_commits(repository_path: &str, old_version: &str, new_version: &str) -> Result<bool, String> {
    let mut clog = try!(Clog::with_dir(repository_path).map_err(|_| "Clog failed".to_owned()));

    let commits = clog.from(format!("v{}", old_version))
        .version(format!("v{}", new_version))
        .get_commits();

    Ok(commits.len() > 0)
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
