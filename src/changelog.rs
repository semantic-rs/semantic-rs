use std::io::BufWriter;
use clog::Clog;
use clog::fmt::MarkdownWriter;
use std::path::PathBuf;
use std::io::prelude::*;
use std::fs::{self, File, OpenOptions};

fn changelog_exists(path: &PathBuf) -> bool {
    match fs::metadata(path) {
        Ok(md) => md.is_file(),
        Err(_) => return false
    }
}

fn prepend_to_file(filename: PathBuf, new_changelog: &str) -> Result<(), String> {
    let mut existing_file_content = String::new();
    let mut f = File::open(&filename).expect("Failed to open Changelog for reading");
    f.read_to_string(&mut existing_file_content).expect("Failed to read Changelog");

    let mut file = OpenOptions::new().create(true).write(true).open(filename).expect("Failed to open Changelog for prepending new items");
    match file.write(format!("{}\n", new_changelog).as_bytes()) {
        Ok(_) => {},
        Err(err) => return Err(format!("Could not prepend text to changelog file: {:?}", err))
    }
    match file.write(format!("{}\n", existing_file_content).as_bytes()) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Could not prepend text to changelog file: {:?}", err))
    }
}

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

pub fn write_custom(repository_path: &str, new_version: &str, changelog_text: &str) -> Result<(), String> {
    let mut changelog_path = PathBuf::from(repository_path);
    changelog_path.push("Changelog.md");
    let changelog_text = format!("## v{}\n{}\n", new_version, changelog_text);
    if changelog_exists(&changelog_path) {
        prepend_to_file(changelog_path, &changelog_text)
    }
    else {
        let mut file = OpenOptions::new().create(true).write(true).open(changelog_path)
            .expect("Failed to create new Changelog");
        match file.write(changelog_text.as_bytes()) {
            Ok(_) => Ok(()),
            Err(err) => Err(format!("Could not prepend text to changelog file: {:?}", err))
        }
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
