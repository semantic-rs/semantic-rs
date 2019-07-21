pub mod clog;
pub mod docker;
pub mod git;
pub mod github;
pub mod rust;

pub use self::clog::ClogPlugin;
pub use self::git::GitPlugin;
pub use self::github::GithubPlugin;
pub use self::rust::RustPlugin;
