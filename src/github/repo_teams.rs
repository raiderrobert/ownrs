use anyhow::Result;
use serde::Deserialize;

use crate::cache::file_cache::FileCache;

use super::client::GitHubClient;

#[derive(Deserialize)]
struct RepoTeam {
    slug: String,
    permission: String,
}

pub async fn fetch_repo_admin_teams(
    client: &GitHubClient,
    org: &str,
    repo: &str,
    cache: &FileCache,
    refresh: bool,
) -> Result<Vec<String>> {
    let cache_key = format!("admin_teams_{org}_{repo}");

    if !refresh {
        if let Some(cached) = cache.get::<Vec<String>>(&cache_key)? {
            return Ok(cached);
        }
    }

    let mut slugs = Vec::new();
    let mut page: u32 = 1;

    loop {
        let route = format!("/repos/{org}/{repo}/teams");
        let result: Result<Vec<RepoTeam>, _> = client
            .octocrab
            .get(&route, Some(&[("per_page", "100"), ("page", &page.to_string())]))
            .await;

        match result {
            Ok(teams) => {
                for team in &teams {
                    if team.permission == "admin" {
                        slugs.push(team.slug.clone());
                    }
                }
                if teams.len() < 100 {
                    break;
                }
                page += 1;
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("403") || err_str.contains("404") {
                    break;
                }
                return Err(e.into());
            }
        }
    }

    cache.set(&cache_key, &slugs)?;
    Ok(slugs)
}
