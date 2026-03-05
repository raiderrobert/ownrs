use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::cache::file_cache::FileCache;

use super::client::GitHubClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    pub name: String,
    pub pushed_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct GraphQLResponse {
    data: Option<GraphQLData>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize)]
struct GraphQLData {
    organization: OrgData,
}

#[derive(Deserialize)]
struct OrgData {
    repositories: RepositoryConnection,
}

#[derive(Deserialize)]
struct RepositoryConnection {
    nodes: Vec<RepoNode>,
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
}

#[derive(Deserialize)]
struct RepoNode {
    name: String,
    #[serde(rename = "pushedAt")]
    pushed_at: Option<String>,
    #[serde(rename = "isArchived")]
    is_archived: bool,
    #[serde(rename = "isFork")]
    is_fork: bool,
}

#[derive(Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
}

#[derive(Deserialize)]
struct GraphQLError {
    message: String,
}

pub async fn list_repos(
    client: &GitHubClient,
    org: &str,
    cache: &FileCache,
    refresh: bool,
    on_progress: impl Fn(usize),
) -> Result<Vec<RepoInfo>> {
    let cache_key = format!("repos_{org}");

    if !refresh {
        if let Some(cached) = cache.get::<Vec<RepoInfo>>(&cache_key)? {
            on_progress(cached.len());
            return Ok(cached);
        }
    }

    let mut repos = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let after = match &cursor {
            Some(c) => format!(r#", after: "{c}""#),
            None => String::new(),
        };

        let query = format!(
            r#"{{
                organization(login: "{org}") {{
                    repositories(first: 100, orderBy: {{field: PUSHED_AT, direction: DESC}}{after}) {{
                        nodes {{
                            name
                            pushedAt
                            isArchived
                            isFork
                        }}
                        pageInfo {{
                            hasNextPage
                            endCursor
                        }}
                    }}
                }}
            }}"#
        );

        let response: GraphQLResponse = client
            .octocrab
            .graphql(&serde_json::json!({ "query": query }))
            .await?;

        if let Some(errors) = response.errors {
            let msgs: Vec<String> = errors.into_iter().map(|e| e.message).collect();
            anyhow::bail!("GraphQL errors: {}", msgs.join(", "));
        }

        let data = response
            .data
            .ok_or_else(|| anyhow::anyhow!("No data in GraphQL response"))?;

        let connection = data.organization.repositories;

        for node in connection.nodes {
            if node.is_archived || node.is_fork {
                continue;
            }

            let pushed_at = node
                .pushed_at
                .and_then(|s| s.parse::<DateTime<Utc>>().ok());

            repos.push(RepoInfo {
                name: node.name,
                pushed_at,
            });
        }

        on_progress(repos.len());

        if !connection.page_info.has_next_page {
            break;
        }
        cursor = connection.page_info.end_cursor;
    }

    cache.set(&cache_key, &repos)?;
    Ok(repos)
}
