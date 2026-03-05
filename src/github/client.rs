use anyhow::Result;
use octocrab::Octocrab;

pub struct GitHubClient {
    pub octocrab: Octocrab,
}

impl GitHubClient {
    pub fn new(token: &str) -> Result<Self> {
        let octocrab = Octocrab::builder()
            .personal_token(token.to_string())
            .build()?;
        Ok(GitHubClient { octocrab })
    }
}
