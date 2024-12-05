use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    // Page title and header
    pub title: String,
    // GitHub token, to circumvent API limits
    pub github_token: String,
    // GitHub owner / organisation
    pub github_owner: String,
    // GitHub repository name
    pub github_repo: String,
    // GitHub client ID and secret for OAuth
    pub github_client_id: String,
    pub github_client_secret: String,
    // OAuth callback URL
    pub authorize_url: String,
    // Session key for cookies
    pub session_key: String,
    // API key for binary uploads
    pub api_key: String,
    // Arguments passed to the binary, use {port} to interpolate the port number
    pub command_args: Vec<String>,
    // Emoji favicon or letter
    pub favicon: String,
    // List of words to combine into a unique service name
    pub words: Vec<String>,
    // Github user handles of admins
    pub admins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<&'static Config> {
        let config: Config = config::Config::builder()
            .add_source(config::File::with_name("config.toml"))
            .add_source(
                config::Environment::with_prefix("etes")
                    .try_parsing(true)
                    .list_separator(" "),
            )
            .build()?
            .try_deserialize()?;

        Ok(Box::leak(Box::new(config)))
    }
}
