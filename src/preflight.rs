use config::Config;

pub fn check(config: &Config) -> Vec<String> {
    let mut warnings = vec!();

    if config.gh_token.is_none() {
        warnings.push("The GH_TOKEN environment variable is not configured".into());
    }

    if let Err(ref err) = config.remote {
        warnings.push(format!("Could not determine the origin remote url: {:?}", err));
        warnings.push("semantic-rs can't push changes or create a release on GitHub".into());
    }

    warnings
}
