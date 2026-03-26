use std::collections::HashMap;

use anyhow::Result;
use serde::Deserialize;

use crate::cache::file_cache::FileCache;

use super::client::GitHubClient;

#[derive(Deserialize)]
struct TeamMember {
    login: String,
}

/// Fetch all team memberships for an org.
/// Returns a map of team_slug -> Vec<username>.
pub async fn fetch_team_members(
    client: &GitHubClient,
    org: &str,
    team_slugs: &[String],
    cache: &FileCache,
    refresh: bool,
) -> Result<HashMap<String, Vec<String>>> {
    let cache_key = format!("team_members_{org}");

    if !refresh {
        if let Some(cached) = cache.get::<HashMap<String, Vec<String>>>(&cache_key)? {
            return Ok(cached);
        }
    }

    let mut membership: HashMap<String, Vec<String>> = HashMap::new();

    for slug in team_slugs {
        let members = fetch_members_for_team(&client.octocrab, org, slug).await;
        membership.insert(slug.clone(), members);
    }

    cache.set(&cache_key, &membership)?;
    Ok(membership)
}

async fn fetch_members_for_team(
    octocrab: &octocrab::Octocrab,
    org: &str,
    team_slug: &str,
) -> Vec<String> {
    let mut logins = Vec::new();
    let mut page: u32 = 1;

    loop {
        let route = format!("/orgs/{org}/teams/{team_slug}/members");
        let result: Result<Vec<TeamMember>, _> = octocrab
            .get(
                &route,
                Some(&[("per_page", "100"), ("page", &page.to_string())]),
            )
            .await;

        match result {
            Ok(members) => {
                let count = members.len();
                for member in members {
                    logins.push(member.login);
                }
                if count < 100 {
                    break;
                }
                page += 1;
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("403") || err_str.contains("404") {
                    break;
                }
                eprintln!("Warning: failed to fetch members for {org}/{team_slug}: {e}");
                break;
            }
        }
    }

    logins
}
