use url::{ParseError, Url};

pub fn user_repo_from_url(url: &str) -> Result<(String, String), String> {
    let path = match Url::parse(url) {
        Err(ParseError::RelativeUrlWithoutBase) => match url.rfind(":") {
            None => return Err("Can't parse path from remote URL".into()),
            Some(colon_pos) => Some(
                url[colon_pos + 1..]
                    .split('/')
                    .map(|s| s.to_owned())
                    .collect::<Vec<_>>(),
            ),
        },
        Err(_) => return Err("Can't parse remote URL".into()),
        Ok(url) => url
            .path_segments()
            .map(|path| path.map(|seg| seg.to_owned()).collect::<Vec<_>>()),
    };

    let path = match path {
        Some(ref path) if path.len() == 2 => path,
        _ => return Err("URL should contain user and repository".into()),
    };

    let user = path[0].clone();
    let repo = match path[1].rfind(".git") {
        None => path[1].clone(),
        Some(suffix_pos) => {
            let valid_pos = path[1].len() - 4;
            if valid_pos == suffix_pos {
                let path = &path[1][0..suffix_pos];
                path.into()
            } else {
                path[1].clone()
            }
        }
    };

    Ok((user, repo))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parses_remote_urls() {
        let urls = [
            "https://github.com/user/repo.git",
            "https://github.com/user/repo",
            "git@github.com:user/repo.git",
            "git@github.com:user/repo",
            "ssh://github.com/user/repo",
            "ssh://github.com/user/repo.git",
        ];

        for url in &urls {
            println!("Testing '{:?}'", url);
            let (user, repo) = user_repo_from_url(url).unwrap();

            assert_eq!("user", user);
            assert_eq!("repo", repo);
        }
    }

    #[test]
    fn parses_other_urls() {
        let urls = [(
            "https://github.com/user/repo.git.repo",
            "user",
            "repo.git.repo",
        )];

        for &(url, exp_user, exp_repo) in &urls {
            println!("Testing '{:?}'", url);
            let (user, repo) = user_repo_from_url(url).unwrap();

            assert_eq!(exp_user, user);
            assert_eq!(exp_repo, repo);
        }
    }

    #[test]
    fn fail_some_urls() {
        let urls = [
            "https://github.com/user",
            "https://github.com/user/repo/issues",
            "://github.com/user/",
        ];

        for url in &urls {
            println!("Testing '{:?}'", url);
            assert!(user_repo_from_url(url).is_err());
        }
    }
}
