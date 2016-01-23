use std::error::Error;
use git2_commit;
use time;

pub fn generate_commit_message(new_version: String) -> String {
    format!("Bump version to {}", new_version).into()
}

pub fn commit_files(repository_path: &String, new_version: String) -> Result<(), String> {
    let files = vec!["Cargo.toml"];
    match git2_commit::add(&repository_path, &files[..]) {
        Ok(_) => {},
        Err(err) => return Err(err.description().into())
    }
    let author = match git2_commit::get_signature() {
        Ok(author) => author,
        Err(err) => return Err(err.description().into())
    };

    match git2_commit::commit(repository_path, &author.name, &author.email, &generate_commit_message(new_version)) {
        Ok(_) => Ok(()),
        Err(err) => Err(err.description().into())
    }
}

pub fn tag(repository_path: &str, new_version: &str) -> Result<(), String> {
    let tag_name = format!("v{}", new_version);

    let date = time::now_utc();
    let now = date.strftime("%Y-%m-%d").expect("Can't format a simple date");
    let tag_message = format!("v{} - {}", new_version, now);

    let author = match git2_commit::get_signature() {
        Ok(author) => author,
        Err(err) => return Err(err.description().into())
    };

    git2_commit::tag(repository_path, &author.name, &author.email, &tag_name, &tag_message)
        .map_err(|err| err.description().into())
}
