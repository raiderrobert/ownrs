use reqwest::StatusCode;
use serde::Deserialize;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::cache::file_cache::FileCache;
use crate::github::client::GitHubClient;
use crate::sources::codeowners;

/// Raw source files fetched for a single repo.
#[derive(Debug, Clone)]
pub struct RepoSources {
    pub repo_name: String,
    pub codeowners: Option<String>,
    pub catalog_info: Option<String>,
    pub admin_teams: Vec<String>,
}

const CODEOWNERS_PATHS: &[&str] = &[".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"];
const CATALOG_PATH: &str = "catalog-info.yaml";

#[derive(Deserialize)]
struct RepoTeam {
    slug: String,
    permission: String,
}

pub async fn fetch_all(
    client: &GitHubClient,
    org: &str,
    repo_names: &[String],
    cache: &FileCache,
    refresh: bool,
) -> Vec<RepoSources> {
    let semaphore = Arc::new(Semaphore::new(20));
    let mut handles = Vec::new();

    for name in repo_names {
        let sem = semaphore.clone();
        let org = org.to_string();
        let name = name.clone();
        let octocrab = client.octocrab.clone();
        let cache_key_co = codeowners_cache_key(&org, &name);
        let cache_key_cat = format!("content_{org}_{name}_catalog");
        let cache_key_admin = format!("admin_teams_{org}_{name}");

        let cached_co: Option<Option<String>> = if !refresh {
            cache.get(&cache_key_co).unwrap_or(None)
        } else {
            None
        };
        let cached_cat: Option<Option<String>> = if !refresh {
            cache.get(&cache_key_cat).unwrap_or(None)
        } else {
            None
        };
        let cached_admin: Option<Vec<String>> = if !refresh {
            cache.get(&cache_key_admin).unwrap_or(None)
        } else {
            None
        };

        // If all are cached, skip the network fetch
        if let (Some(co), Some(cat), Some(admin)) =
            (cached_co.clone(), cached_cat.clone(), cached_admin.clone())
        {
            handles.push(tokio::spawn(async move {
                RepoSources {
                    repo_name: name,
                    codeowners: co,
                    catalog_info: cat,
                    admin_teams: admin,
                }
            }));
            continue;
        }

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let codeowners = match cached_co {
                Some(v) => v,
                None => fetch_codeowners(&octocrab, &org, &name).await,
            };

            let catalog_info = match cached_cat {
                Some(v) => v,
                None => fetch_file_content(&octocrab, &org, &name, CATALOG_PATH).await,
            };

            let admin_teams = match cached_admin {
                Some(v) => v,
                None => fetch_admin_teams(&octocrab, &org, &name).await,
            };

            RepoSources {
                repo_name: name,
                codeowners,
                catalog_info,
                admin_teams,
            }
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        if let Ok(sources) = handle.await {
            let cache_key_co = codeowners_cache_key(org, &sources.repo_name);
            let cache_key_cat = format!("content_{org}_{}_catalog", sources.repo_name);
            let cache_key_admin = format!("admin_teams_{org}_{}", sources.repo_name);
            let _ = cache.set(&cache_key_co, &sources.codeowners);
            let _ = cache.set(&cache_key_cat, &sources.catalog_info);
            let _ = cache.set(&cache_key_admin, &sources.admin_teams);

            results.push(sources);
        }
    }
    results
}

fn codeowners_cache_key(org: &str, repo: &str) -> String {
    format!("content_{org}_{repo}_codeowners_v3")
}

/// Resolve which CODEOWNERS file to use, walking `paths` in GitHub's precedence order.
///
/// Prefers the first file GitHub would actually assign reviewers from. If every
/// file present is unusable — an empty file, or a bare owner list that lost its
/// `*` pattern — returns the first one found, so callers can still tell
/// "found but unusable" from "absent".
async fn resolve_codeowners<F, Fut>(paths: &[&str], mut fetch: F) -> Option<String>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Option<String>>,
{
    let mut fallback = None;

    for path in paths {
        let Some(content) = fetch(path.to_string()).await else {
            continue;
        };

        if codeowners::has_usable_rule(&content) {
            return Some(content);
        }

        if fallback.is_none() {
            fallback = Some(content);
        }
    }

    fallback
}

async fn fetch_codeowners(octocrab: &octocrab::Octocrab, org: &str, repo: &str) -> Option<String> {
    resolve_codeowners(CODEOWNERS_PATHS, |path| async move {
        fetch_file_content(octocrab, org, repo, &path).await
    })
    .await
}

async fn fetch_file_content(
    octocrab: &octocrab::Octocrab,
    org: &str,
    repo: &str,
    path: &str,
) -> Option<String> {
    let result = octocrab
        .repos(org, repo)
        .get_content()
        .path(path)
        .r#ref("HEAD")
        .send()
        .await;

    match result {
        Ok(content) => content
            .items
            .first()
            .and_then(|item| item.decoded_content()),
        Err(_) => None,
    }
}

async fn fetch_admin_teams(octocrab: &octocrab::Octocrab, org: &str, repo: &str) -> Vec<String> {
    let mut slugs = Vec::new();
    let mut page: u32 = 1;

    loop {
        let route = format!("/repos/{org}/{repo}/teams");
        let result: Result<Vec<RepoTeam>, _> = octocrab
            .get(
                &route,
                Some(&[("per_page", "100"), ("page", &page.to_string())]),
            )
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
                match &e {
                    octocrab::Error::GitHub { source, .. }
                        if source.status_code == StatusCode::FORBIDDEN
                            || source.status_code == StatusCode::NOT_FOUND =>
                    {
                        // Repo not accessible with this token — expected, skip silently
                    }
                    _ => {
                        eprintln!("Warning: failed to fetch teams for {org}/{repo}: {e}");
                    }
                }
                break;
            }
        }
    }

    slugs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Fake fetcher over a fixed path -> content map, recording lookup order.
    fn fake<'a>(
        files: &'a [(&'a str, &'a str)],
        seen: &'a RefCell<Vec<String>>,
    ) -> impl FnMut(String) -> std::future::Ready<Option<String>> + 'a {
        move |path: String| {
            seen.borrow_mut().push(path.clone());
            let hit = files
                .iter()
                .find(|(p, _)| *p == path)
                .map(|(_, c)| c.to_string());
            std::future::ready(hit)
        }
    }

    type Case = (
        &'static str,
        &'static [(&'static str, &'static str)],
        Option<&'static str>,
        &'static [&'static str],
    );

    #[tokio::test]
    async fn resolves_codeowners_in_github_precedence_order() {
        let cases: &[Case] = &[
            (
                "prefers .github over root and stops early",
                &[
                    (".github/CODEOWNERS", "* @acme/team-a\n"),
                    ("CODEOWNERS", "* @acme/team-b\n"),
                ],
                Some("* @acme/team-a\n"),
                &[".github/CODEOWNERS"],
            ),
            (
                "keeps a .github file that names only users",
                &[
                    (".github/CODEOWNERS", "* @alice\n"),
                    ("CODEOWNERS", "* @acme/team-b\n"),
                ],
                Some("* @alice\n"),
                &[".github/CODEOWNERS"],
            ),
            (
                "uses root when .github is absent",
                &[
                    ("CODEOWNERS", "* @acme/team-b\n"),
                    ("docs/CODEOWNERS", "* @acme/team-c\n"),
                ],
                Some("* @acme/team-b\n"),
                &[".github/CODEOWNERS", "CODEOWNERS"],
            ),
            (
                "falls through past an empty .github file",
                &[
                    (".github/CODEOWNERS", ""),
                    ("CODEOWNERS", "* @acme/team-b\n"),
                ],
                Some("* @acme/team-b\n"),
                &[".github/CODEOWNERS", "CODEOWNERS"],
            ),
            (
                "falls through when .github has no usable rule",
                &[
                    (".github/CODEOWNERS", "@acme/team-a @acme/team-b\n"),
                    ("CODEOWNERS", "* @acme/team-a\n"),
                ],
                Some("* @acme/team-a\n"),
                &[".github/CODEOWNERS", "CODEOWNERS"],
            ),
            (
                "skips absent paths",
                &[("docs/CODEOWNERS", "* @acme/team-c\n")],
                Some("* @acme/team-c\n"),
                &[".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"],
            ),
            (
                "keeps first found when nothing is usable",
                &[
                    (".github/CODEOWNERS", "@acme/team-a\n"),
                    ("CODEOWNERS", "# only a comment\n"),
                ],
                Some("@acme/team-a\n"),
                &[".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"],
            ),
            (
                "none when no file exists",
                &[],
                None,
                &[".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"],
            ),
        ];

        for (name, files, want, want_seen) in cases {
            let seen = RefCell::new(Vec::new());
            let got = resolve_codeowners(CODEOWNERS_PATHS, fake(files, &seen)).await;

            assert_eq!(got.as_deref(), *want, "{name}");
            assert_eq!(seen.borrow().as_slice(), *want_seen, "{name}");
        }
    }
}
