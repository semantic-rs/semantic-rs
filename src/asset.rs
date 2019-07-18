use std::path::{Path, PathBuf};
use failure::Error;

#[derive(Clone, Debug)]
pub struct Asset {
    path: PathBuf,
    name: String,
    content_type: String,
}

impl Asset {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();

        // Check if path exists
        if !path.exists() {
            return Err(failure::format_err!(
                "asset file not found at {}",
                path.display()
            ));
        }

        // Check is asset is file
        if !path.is_file() {
            return Err(failure::format_err!(
                "asset at {} is not a file",
                path.display()
            ));
        }

        // Create a name from the file path
        let name = path
            .file_name()
            .ok_or_else(|| failure::format_err!("couldn't get a file stem for {}", path.display()))?
            .to_str()
            .ok_or_else(|| {
                failure::format_err!("{} is not a valid utf-8 path name", path.display())
            })?
            .to_owned();

        // Extract the content type
        let content_type = tree_magic::from_filepath(&path);

        Ok(Asset {
            path,
            name,
            content_type,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn content_type(&self) -> &str {
        &self.content_type
    }
}
