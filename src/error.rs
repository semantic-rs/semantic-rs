use git2::Error as GitError;
use github_rs::errors::Error as GithubError;
use hubcaps::Error as HubcapsError;
use std::env::VarError;
use std::error::Error as StdError;
use std::io::Error as IoError;

use std::convert::From;
use std::fmt;

use self::Error::*;
use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    Git(GitError),
    Var(VarError),
    Io(IoError),
    Hubcaps(HubcapsError),
    GitHub(GithubError),
    Custom(String),
}

impl From<GitError> for Error {
    fn from(err: GitError) -> Error {
        Error::Git(err)
    }
}

impl From<VarError> for Error {
    fn from(err: VarError) -> Error {
        Error::Var(err)
    }
}

impl From<IoError> for Error {
    fn from(err: IoError) -> Error {
        Error::Io(err)
    }
}

impl From<HubcapsError> for Error {
    fn from(err: HubcapsError) -> Error {
        Error::Hubcaps(err)
    }
}

impl From<GithubError> for Error {
    fn from(err: GithubError) -> Self {
        Error::GitHub(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Git(ref e) => Display::fmt(e, f),
            Var(ref e) => Display::fmt(e, f),
            Io(ref e) => Display::fmt(e, f),
            Hubcaps(ref e) => Display::fmt(e, f),
            GitHub(ref e) => Display::fmt(e, f),
            Custom(ref e) => Display::fmt(e, f),
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Git(ref e) => e.description(),
            Var(ref e) => e.description(),
            Io(ref e) => e.description(),
            Hubcaps(ref e) => e.description(),
            GitHub(ref e) => e.description(),
            Custom(ref e) => e.as_str(),
        }
    }
}
