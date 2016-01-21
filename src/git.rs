use std::error::Error;
use git2_commit;

pub fn generate_commit_message(new_version: String) -> String {
    format!("Bump version to {}", new_version).into()
}

pub fn commit_files(repository_path: &String, new_version: String) -> Result<(), String> {
    let files = vec!["Cargo.toml", "Cargo.lock"];
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
