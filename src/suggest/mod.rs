pub mod types;

use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::cache::file_cache::FileCache;
use crate::github::client::GitHubClient;
use types::{SuggestionResult, TeamSuggestion};

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

/// Build a reverse lookup: username -> list of team slugs.
fn build_user_to_teams(team_members: &HashMap<String, Vec<String>>) -> HashMap<String, Vec<String>> {
    let mut user_to_teams: HashMap<String, Vec<String>> = HashMap::new();
    for (team, members) in team_members {
        for member in members {
            user_to_teams
                .entry(member.clone())
                .or_default()
                .push(team.clone());
        }
    }
    user_to_teams
}

/// Score teams based on contributor activity.
pub fn score_teams(
    team_members: &HashMap<String, Vec<String>>,
    commit_authors: &HashMap<String, usize>,
    pr_reviewers: &HashMap<String, usize>,
    lookback_days: u64,
) -> SuggestionResult {
    let user_to_teams = build_user_to_teams(team_members);

    // Collect all unique contributors
    let mut all_users: std::collections::HashSet<String> = std::collections::HashSet::new();
    for user in commit_authors.keys() {
        all_users.insert(user.clone());
    }
    for user in pr_reviewers.keys() {
        all_users.insert(user.clone());
    }

    // Per-team accumulators
    let mut team_commits: HashMap<String, usize> = HashMap::new();
    let mut team_reviews: HashMap<String, usize> = HashMap::new();
    let mut team_active_members: HashMap<String, Vec<String>> = HashMap::new();
    let mut unresolved: Vec<String> = Vec::new();

    for user in &all_users {
        let commits = commit_authors.get(user).copied().unwrap_or(0);
        let reviews = pr_reviewers.get(user).copied().unwrap_or(0);

        match user_to_teams.get(user) {
            Some(teams) => {
                for team in teams {
                    *team_commits.entry(team.clone()).or_insert(0) += commits;
                    *team_reviews.entry(team.clone()).or_insert(0) += reviews;
                    let members = team_active_members.entry(team.clone()).or_default();
                    if !members.contains(user) {
                        members.push(user.clone());
                    }
                }
            }
            None => {
                unresolved.push(user.clone());
            }
        }
    }

    // Build suggestions from teams that had any activity
    let mut suggestions: Vec<TeamSuggestion> = Vec::new();
    for (team, members) in &team_active_members {
        let commits = team_commits.get(team).copied().unwrap_or(0);
        let reviews = team_reviews.get(team).copied().unwrap_or(0);
        if commits > 0 || reviews > 0 {
            suggestions.push(TeamSuggestion {
                team: team.clone(),
                commits,
                reviews,
                members: members.clone(),
            });
        }
    }

    // Sort: total activity desc, then member count desc
    suggestions.sort_by(|a, b| {
        let a_total = a.commits + a.reviews;
        let b_total = b.commits + b.reviews;
        b_total
            .cmp(&a_total)
            .then(b.members.len().cmp(&a.members.len()))
    });

    // Limit to top 3
    suggestions.truncate(3);

    unresolved.sort();

    SuggestionResult {
        suggestions,
        unresolved,
        lookback_days,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_team_members() -> HashMap<String, Vec<String>> {
        let mut m = HashMap::new();
        m.insert("team-platform".to_string(), vec!["alice".to_string(), "bob".to_string()]);
        m.insert("team-infra".to_string(), vec!["charlie".to_string()]);
        m
    }

    #[test]
    fn scores_teams_from_activity() {
        let team_members = make_team_members();
        let mut commit_authors = HashMap::new();
        commit_authors.insert("alice".to_string(), 3);
        commit_authors.insert("bob".to_string(), 2);
        commit_authors.insert("charlie".to_string(), 1);

        let pr_reviewers = HashMap::new();

        let result = score_teams(&team_members, &commit_authors, &pr_reviewers, 90);

        assert_eq!(result.suggestions.len(), 2);
        assert_eq!(result.suggestions[0].team, "team-platform");
        assert_eq!(result.suggestions[0].commits, 5);
        assert_eq!(result.suggestions[0].reviews, 0);
        assert_eq!(result.suggestions[0].members.len(), 2);
        assert_eq!(result.suggestions[1].team, "team-infra");
        assert_eq!(result.suggestions[1].commits, 1);
        assert!(result.unresolved.is_empty());
    }

    #[test]
    fn unresolved_contributors_tracked() {
        let team_members = make_team_members();
        let mut commit_authors = HashMap::new();
        commit_authors.insert("alice".to_string(), 1);
        commit_authors.insert("external-dev".to_string(), 5);

        let pr_reviewers = HashMap::new();

        let result = score_teams(&team_members, &commit_authors, &pr_reviewers, 90);

        assert_eq!(result.suggestions.len(), 1);
        assert_eq!(result.suggestions[0].team, "team-platform");
        assert!(result.unresolved.contains(&"external-dev".to_string()));
    }

    #[test]
    fn reviews_counted_in_scoring() {
        let team_members = make_team_members();
        let commit_authors = HashMap::new();
        let mut pr_reviewers = HashMap::new();
        pr_reviewers.insert("charlie".to_string(), 5);

        let result = score_teams(&team_members, &commit_authors, &pr_reviewers, 90);

        assert_eq!(result.suggestions[0].team, "team-infra");
        assert_eq!(result.suggestions[0].reviews, 5);
        assert_eq!(result.suggestions[0].commits, 0);
    }

    #[test]
    fn person_in_multiple_teams_counts_for_all() {
        let mut team_members = HashMap::new();
        team_members.insert("team-a".to_string(), vec!["alice".to_string()]);
        team_members.insert("team-b".to_string(), vec!["alice".to_string()]);

        let mut commit_authors = HashMap::new();
        commit_authors.insert("alice".to_string(), 3);

        let pr_reviewers = HashMap::new();

        let result = score_teams(&team_members, &commit_authors, &pr_reviewers, 90);

        assert_eq!(result.suggestions.len(), 2);
        for suggestion in &result.suggestions {
            assert_eq!(suggestion.commits, 3);
        }
    }

    #[test]
    fn top_3_limit() {
        let mut team_members = HashMap::new();
        team_members.insert("team-a".to_string(), vec!["a".to_string()]);
        team_members.insert("team-b".to_string(), vec!["b".to_string()]);
        team_members.insert("team-c".to_string(), vec!["c".to_string()]);
        team_members.insert("team-d".to_string(), vec!["d".to_string()]);

        let mut commit_authors = HashMap::new();
        commit_authors.insert("a".to_string(), 4);
        commit_authors.insert("b".to_string(), 3);
        commit_authors.insert("c".to_string(), 2);
        commit_authors.insert("d".to_string(), 1);

        let pr_reviewers = HashMap::new();

        let result = score_teams(&team_members, &commit_authors, &pr_reviewers, 90);

        assert_eq!(result.suggestions.len(), 3);
        assert_eq!(result.suggestions[0].team, "team-a");
        assert_eq!(result.suggestions[2].team, "team-c");
    }

    #[test]
    fn no_activity_returns_empty() {
        let team_members = make_team_members();
        let commit_authors = HashMap::new();
        let pr_reviewers = HashMap::new();

        let result = score_teams(&team_members, &commit_authors, &pr_reviewers, 90);

        assert!(result.suggestions.is_empty());
        assert!(result.unresolved.is_empty());
    }
}
