use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::cache::file_cache::FileCache;
use crate::cli::{RepoExclude, Visibility};

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
    #[serde(rename = "isEmpty")]
    is_empty: bool,
    #[serde(rename = "isTemplate")]
    is_template: bool,
    #[serde(rename = "isMirror")]
    is_mirror: bool,
    visibility: String,
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

/// Which repos an org listing should include.
pub struct RepoFilters<'a> {
    exclude: &'a [RepoExclude],
    visibility: &'a [Visibility],
}

impl<'a> RepoFilters<'a> {
    pub fn new(exclude: &'a [RepoExclude], visibility: &'a [Visibility]) -> Self {
        // `--exclude none` turns every exclusion off, whatever else was listed.
        let exclude = if exclude.contains(&RepoExclude::None) {
            &[][..]
        } else {
            exclude
        };
        RepoFilters {
            exclude,
            visibility,
        }
    }

    fn admits(&self, node: &RepoNode) -> bool {
        let excluded_kind = [
            (node.is_archived, RepoExclude::Archived),
            (node.is_fork, RepoExclude::Forks),
            (node.is_empty, RepoExclude::Empty),
            (node.is_template, RepoExclude::Template),
            (node.is_mirror, RepoExclude::Mirror),
        ]
        .iter()
        .any(|(flag, kind)| *flag && self.exclude.contains(kind));

        if excluded_kind {
            return false;
        }

        // An empty visibility list means every visibility.
        self.visibility.is_empty()
            || self
                .visibility
                .iter()
                .any(|v| node.visibility.eq_ignore_ascii_case(v.as_str()))
    }

    /// Cache-key suffix. Listings under different filters must not share an entry.
    fn cache_suffix(&self) -> String {
        fn joined(mut names: Vec<&str>, empty: &str) -> String {
            if names.is_empty() {
                return empty.to_string();
            }
            names.sort_unstable();
            names.join("-")
        }

        let exclude = joined(
            self.exclude.iter().map(RepoExclude::as_str).collect(),
            "none",
        );
        let visibility = joined(
            self.visibility.iter().map(Visibility::as_str).collect(),
            "all",
        );
        format!("ex_{exclude}_vis_{visibility}")
    }
}

pub async fn list_repos(
    client: &GitHubClient,
    org: &str,
    cache: &FileCache,
    refresh: bool,
    filters: &RepoFilters<'_>,
    on_progress: impl Fn(usize),
) -> Result<Vec<RepoInfo>> {
    let cache_key = format!("repos_{org}_{}", filters.cache_suffix());

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
                            isEmpty
                            isTemplate
                            isMirror
                            visibility
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
            if !filters.admits(&node) {
                continue;
            }

            let pushed_at = node.pushed_at.and_then(|s| s.parse::<DateTime<Utc>>().ok());

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

#[cfg(test)]
mod tests {
    use super::*;

    fn node(visibility: &str) -> RepoNode {
        RepoNode {
            name: "repo".to_string(),
            pushed_at: None,
            is_archived: false,
            is_fork: false,
            is_empty: false,
            is_template: false,
            is_mirror: false,
            visibility: visibility.to_string(),
        }
    }

    #[test]
    fn default_excludes_archived_and_forks() {
        let exclude = [RepoExclude::Archived, RepoExclude::Forks];
        let filters = RepoFilters::new(&exclude, &[]);

        assert!(filters.admits(&node("PUBLIC")));

        let mut archived = node("PUBLIC");
        archived.is_archived = true;
        assert!(!filters.admits(&archived));

        let mut fork = node("PUBLIC");
        fork.is_fork = true;
        assert!(!filters.admits(&fork));
    }

    #[test]
    fn unlisted_kinds_are_admitted() {
        let exclude = [RepoExclude::Archived];
        let filters = RepoFilters::new(&exclude, &[]);

        let mut fork = node("PUBLIC");
        fork.is_fork = true;
        assert!(filters.admits(&fork));
    }

    #[test]
    fn none_overrides_every_other_exclusion() {
        let exclude = [RepoExclude::Archived, RepoExclude::None];
        let filters = RepoFilters::new(&exclude, &[]);

        let mut archived = node("PUBLIC");
        archived.is_archived = true;
        archived.is_fork = true;
        assert!(filters.admits(&archived));
    }

    #[test]
    fn empty_template_and_mirror_are_filterable() {
        for (set_flag, kind) in [
            (
                (|n: &mut RepoNode| n.is_empty = true) as fn(&mut RepoNode),
                RepoExclude::Empty,
            ),
            (
                |n: &mut RepoNode| n.is_template = true,
                RepoExclude::Template,
            ),
            (|n: &mut RepoNode| n.is_mirror = true, RepoExclude::Mirror),
        ] {
            let exclude = [kind];
            let filters = RepoFilters::new(&exclude, &[]);

            let mut repo = node("PUBLIC");
            set_flag(&mut repo);
            assert!(
                !filters.admits(&repo),
                "{} should be excluded",
                kind.as_str()
            );
            assert!(filters.admits(&node("PUBLIC")));
        }
    }

    #[test]
    fn no_visibility_filter_admits_all_visibilities() {
        let filters = RepoFilters::new(&[], &[]);
        for v in ["PUBLIC", "PRIVATE", "INTERNAL"] {
            assert!(filters.admits(&node(v)));
        }
    }

    #[test]
    fn visibility_filter_is_case_insensitive() {
        let visibility = [Visibility::Private];
        let filters = RepoFilters::new(&[], &visibility);

        assert!(filters.admits(&node("PRIVATE")));
        assert!(filters.admits(&node("private")));
        assert!(!filters.admits(&node("PUBLIC")));
    }

    #[test]
    fn visibility_filter_accepts_any_listed_value() {
        let visibility = [Visibility::Private, Visibility::Internal];
        let filters = RepoFilters::new(&[], &visibility);

        assert!(filters.admits(&node("PRIVATE")));
        assert!(filters.admits(&node("INTERNAL")));
        assert!(!filters.admits(&node("PUBLIC")));
    }

    #[test]
    fn cache_suffix_is_order_independent() {
        let a = [RepoExclude::Forks, RepoExclude::Archived];
        let b = [RepoExclude::Archived, RepoExclude::Forks];
        assert_eq!(
            RepoFilters::new(&a, &[]).cache_suffix(),
            RepoFilters::new(&b, &[]).cache_suffix()
        );
    }

    #[test]
    fn cache_suffix_distinguishes_filter_sets() {
        let archived = [RepoExclude::Archived];
        let both = [RepoExclude::Archived, RepoExclude::Forks];
        let private = [Visibility::Private];

        let keys = [
            RepoFilters::new(&archived, &[]).cache_suffix(),
            RepoFilters::new(&both, &[]).cache_suffix(),
            RepoFilters::new(&archived, &private).cache_suffix(),
            RepoFilters::new(&[], &[]).cache_suffix(),
        ];

        let unique: std::collections::HashSet<&String> = keys.iter().collect();
        assert_eq!(unique.len(), keys.len(), "suffixes collided: {keys:?}");
    }

    #[test]
    fn cache_suffix_names_the_empty_cases() {
        assert_eq!(RepoFilters::new(&[], &[]).cache_suffix(), "ex_none_vis_all");
        assert_eq!(
            RepoFilters::new(&[RepoExclude::None], &[]).cache_suffix(),
            "ex_none_vis_all"
        );
    }
}
