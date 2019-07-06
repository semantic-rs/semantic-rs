use crate::config::Config;
use crate::error::Error;
use crate::git;

pub fn check(config: &Config) -> Vec<String> {
    let mut warnings = vec![];

    if config.gh_token.is_none() {
        warnings.push("The GH_TOKEN environment variable is not configured".into());
    }

    if config.cargo_token.is_none() {
        warnings.push("The CARGO_TOKEN environment variable is not configured. Cannot create release on crates.io".into());
    }

    if let Err(ref err) = config.remote {
        warnings.push(format!("Could not determine the origin remote: {:?}", err));
        warnings.push("semantic-rs can't push changes or create a release on GitHub".into());
    } else {
        let remote_name = config.remote.as_ref().unwrap();
        let remote_url = crate::git::get_remote_url(config, remote_name);
        match remote_url {
            Ok(Some(remote_url)) => {
                log::info!("Current remote: {}({})", remote_name, remote_url);
                if !git::is_https_remote(Some(&remote_url)) && !config.force_https {
                    warnings.push("Git remote is not HTTPS:".into());
                    warnings.push("The publishing will fail if your environment doesn't hold your git ssh keys".into());
                    warnings.push("Consider adding --force-https flag, that's most likely what you want if you're using GH_TOKEN authentication".into());
                }
            }
            Ok(None) => log::info!("Current remote: {}", remote_name),
            Err(err) => warnings.push(format!(
                "Could not determine the origin remote url: {}",
                err
            )),
        }
    }

    warnings
}

pub fn apply_overrides(config: &mut Config) -> Result<(), Error> {
    if config.force_https {
        let remote_name = config.remote.clone().unwrap();
        let remote_url = config
            .repository
            .find_remote(&remote_name)?
            .url()
            .map(str::to_string);

        let remote_url = remote_url.unwrap_or_else(|| {
            log::error!(
                "remote url for {} is not found in the repository",
                remote_name
            );
            std::process::exit(1);
        });

        if !git::is_https_remote(Some(&remote_url)) {
            // TODO: replace with generic regex
            let rules = [
                ("git@github.com:", "https://github.com/"),
                ("git://github.com/", "https://github.com/"),
            ];

            let mut new_url = None;

            for (pattern, substitute) in &rules {
                if remote_url.starts_with(pattern) {
                    new_url = Some(remote_url.replace(pattern, substitute));
                    break;
                }
            }

            if new_url.is_none() {
                log::error!("{} is not supported for https forcing, please consider opening an issue at https://github.com/etclabscore/semantic-rs/issues/new/choose", remote_url);
                std::process::exit(1);
            }

            match new_url {
                None => {
                    log::error!("{} is not supported for https forcing, please consider opening an issue at https://github.com/etclabscore/semantic-rs/issues/new/choose", remote_url);
                    std::process::exit(1);
                }
                Some(new_url) => {
                    log::info!("Overriding git remote url: {} -> {}", remote_url, new_url);
                    git::set_remote_url(config, &remote_name, &new_url)?;
                }
            }
        }
    }

    Ok(())
}
