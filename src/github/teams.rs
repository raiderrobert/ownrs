use std::collections::HashSet;

use anyhow::Result;

use crate::cache::file_cache::FileCache;

use super::client::GitHubClient;

pub async fn fetch_team_slugs(
    client: &GitHubClient,
    org: &str,
    cache: &FileCache,
    refresh: bool,
) -> Result<HashSet<String>> {
    let cache_key = format!("teams_{org}");

    if !refresh {
        if let Some(cached) = cache.get::<Vec<String>>(&cache_key)? {
            return Ok(cached.into_iter().collect());
        }
    }

    let mut slugs = Vec::new();
    let mut page: u32 = 1;

    loop {
        let teams = client
            .octocrab
            .teams(org)
            .list()
            .per_page(100)
            .page(page)
            .send()
            .await?;

        let items = teams.items;
        if items.is_empty() {
            break;
        }

        for team in &items {
            slugs.push(team.slug.clone());
        }

        if teams.next.is_none() {
            break;
        }
        page += 1;
    }

    cache.set(&cache_key, &slugs)?;
    Ok(slugs.into_iter().collect())
}
