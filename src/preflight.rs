use config::Config;

pub fn check_for_github_release(config: &Config) -> Vec<String> {
    let mut warnings = vec!();

    if config.gh_token.is_none() {
        warnings.push("The GH_TOKEN environment variable is not configured".into());
    }

    warnings
}
