pub mod request;
pub mod response;

pub type GitRevision = String;

pub type Null = ();

pub type ReleaseNotes = String;

pub type MethodName = String;

pub type Warning = String;

pub type Error = String;

#[derive(Clone, Debug)]
pub struct Version {
    pub rev: GitRevision,
    pub semver: Option<semver::Version>,
}
