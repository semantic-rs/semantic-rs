use clog::Clog;

pub fn write(repository_path: &str, old_version: &str, new_version: &str) -> Result<(), String> {
    let mut clog = try!(Clog::with_dir(repository_path).map_err(|_| "Clog failed".to_owned()));

    // TODO: Make this configurable? Rely on clog's own configuration?
    clog.changelog("Changelog.md")
        .from(format!("v{}", old_version))
        .version(format!("v{}", new_version));

    clog.write_changelog().map_err(|_| "Failed to write Changelog.md".to_owned())
}
