pub mod types;

use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::cache::file_cache::FileCache;
use crate::github::client::GitHubClient;

#[derive(Deserialize)]
struct CommitResponse {
    author: Option<CommitAuthor>,
}

#[derive(Deserialize)]
struct CommitAuthor {
    login: String,
}

#[derive(Deserialize)]
struct PullRequest {
    number: u64,
    updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct Review {
    user: Option<ReviewUser>,
}

#[derive(Deserialize)]
struct ReviewUser {
    login: String,
}

/// Fetch recent commit authors for a repo, returning username -> commit count.
pub async fn fetch_commit_authors(
    client: &GitHubClient,
    org: &str,
    repo: &str,
    since: &DateTime<Utc>,
    cache: &FileCache,
    refresh: bool,
) -> Result<HashMap<String, usize>> {
    let cache_key = format!("suggest_commits_{org}_{repo}");

    if !refresh {
        if let Some(cached) = cache.get::<HashMap<String, usize>>(&cache_key)? {
            return Ok(cached);
        }
    }

    let mut authors: HashMap<String, usize> = HashMap::new();
    let since_str = since.to_rfc3339();
    let mut page: u32 = 1;

    loop {
        let route = format!("/repos/{org}/{repo}/commits");
        let result: Result<Vec<CommitResponse>, _> = client
            .octocrab
            .get(
                &route,
                Some(&[
                    ("since", since_str.as_str()),
                    ("per_page", "100"),
                    ("page", &page.to_string()),
                ]),
            )
            .await;

        match result {
            Ok(commits) => {
                let count = commits.len();
                for commit in commits {
                    if let Some(author) = commit.author {
                        *authors.entry(author.login).or_insert(0) += 1;
                    }
                }
                if count < 100 {
                    break;
                }
                page += 1;
            }
            Err(_) => break,
        }
    }

    cache.set(&cache_key, &authors)?;
    Ok(authors)
}

/// Fetch recent PR reviewers for a repo, returning username -> review count.
pub async fn fetch_pr_reviewers(
    client: &GitHubClient,
    org: &str,
    repo: &str,
    since: &DateTime<Utc>,
    cache: &FileCache,
    refresh: bool,
) -> Result<HashMap<String, usize>> {
    let cache_key = format!("suggest_reviews_{org}_{repo}");

    if !refresh {
        if let Some(cached) = cache.get::<HashMap<String, usize>>(&cache_key)? {
            return Ok(cached);
        }
    }

    let mut reviewers: HashMap<String, usize> = HashMap::new();

    // Fetch recently-updated closed PRs
    let mut page: u32 = 1;
    let mut pr_numbers: Vec<u64> = Vec::new();

    loop {
        let route = format!("/repos/{org}/{repo}/pulls");
        let result: Result<Vec<PullRequest>, _> = client
            .octocrab
            .get(
                &route,
                Some(&[
                    ("state", "closed"),
                    ("sort", "updated"),
                    ("direction", "desc"),
                    ("per_page", "100"),
                    ("page", &page.to_string()),
                ]),
            )
            .await;

        match result {
            Ok(prs) => {
                let count = prs.len();
                for pr in prs {
                    if pr.updated_at >= *since {
                        pr_numbers.push(pr.number);
                    } else {
                        // PRs are sorted by updated desc, so we can stop
                        break;
                    }
                }
                if count < 100 {
                    break;
                }
                page += 1;
            }
            Err(_) => break,
        }
    }

    // Fetch reviews for each PR
    for pr_number in pr_numbers {
        let route = format!("/repos/{org}/{repo}/pulls/{pr_number}/reviews");
        let result: Result<Vec<Review>, _> = client
            .octocrab
            .get(&route, None::<&[(&str, &str)]>)
            .await;

        if let Ok(reviews) = result {
            for review in reviews {
                if let Some(user) = review.user {
                    *reviewers.entry(user.login).or_insert(0) += 1;
                }
            }
        }
    }

    cache.set(&cache_key, &reviewers)?;
    Ok(reviewers)
}
