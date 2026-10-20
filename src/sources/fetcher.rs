use serde::Deserialize;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::cache::file_cache::FileCache;
use crate::github::client::GitHubClient;
use crate::github::failure::{classify, describe, Failure};
use crate::sources::codeowners;

/// Raw source files fetched for a single repo.
#[derive(Debug, Clone)]
pub struct RepoSources {
    pub repo_name: String,
    pub codeowners: Option<String>,
    pub catalog_info: Option<String>,
    pub admin_teams: Vec<String>,
}

enum Outcome {
    Cached(RepoSources),
    Fetched(RepoSources),
    Failed(String, octocrab::Error),
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
) -> anyhow::Result<Vec<RepoSources>> {
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

        if let (Some(co), Some(cat), Some(admin)) =
            (cached_co.clone(), cached_cat.clone(), cached_admin.clone())
        {
            handles.push(tokio::spawn(async move {
                Outcome::Cached(RepoSources {
                    repo_name: name,
                    codeowners: co,
                    catalog_info: cat,
                    admin_teams: admin,
                })
            }));
            continue;
        }

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let fetched = async {
                let codeowners = match cached_co {
                    Some(v) => v,
                    None => fetch_codeowners(&octocrab, &org, &name).await?,
                };
                let catalog_info = match cached_cat {
                    Some(v) => v,
                    None => fetch_file_content(&octocrab, &org, &name, CATALOG_PATH).await?,
                };
                let admin_teams = match cached_admin {
                    Some(v) => v,
                    None => fetch_admin_teams(&octocrab, &org, &name).await?,
                };
                Ok::<_, octocrab::Error>((codeowners, catalog_info, admin_teams))
            };

            match fetched.await {
                Ok((codeowners, catalog_info, admin_teams)) => Outcome::Fetched(RepoSources {
                    repo_name: name,
                    codeowners,
                    catalog_info,
                    admin_teams,
                }),
                Err(error) => Outcome::Failed(name, error),
            }
        }));
    }

    let mut results = Vec::new();
    let mut failures = Vec::new();
    for handle in handles {
        match handle.await? {
            Outcome::Cached(sources) => results.push(sources),
            Outcome::Fetched(sources) => {
                let name = &sources.repo_name;
                let _ = cache.set(&codeowners_cache_key(org, name), &sources.codeowners);
                let _ = cache.set(
                    &format!("content_{org}_{name}_catalog"),
                    &sources.catalog_info,
                );
                let _ = cache.set(&format!("admin_teams_{org}_{name}"), &sources.admin_teams);
                results.push(sources);
            }
            Outcome::Failed(name, error) => failures.push((name, error)),
        }
    }

    if let Some((name, error)) = failures.first() {
        anyhow::bail!(
            "failed to fetch ownership sources for {} of {} repos (first: {org}/{name}: {}). \
             Nothing was reported. Fetched repos are cached; rerun without --refresh to retry the rest.",
            failures.len(),
            repo_names.len(),
            describe(error),
        );
    }

    Ok(results)
}

fn codeowners_cache_key(org: &str, repo: &str) -> String {
    format!("content_{org}_{repo}_codeowners_v3")
}

/// Resolve which CODEOWNERS file to use, walking `paths` in GitHub's precedence order.
///
/// Prefers the first file GitHub would actually assign reviewers from. If every
/// file present is unusable — an empty file, or a bare owner list that lost its
/// `*` pattern — returns the first one found, so callers can still tell
/// "found but unusable" from "absent". A failed fetch stops the walk.
async fn resolve_codeowners<F, Fut, E>(paths: &[&str], mut fetch: F) -> Result<Option<String>, E>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<Option<String>, E>>,
{
    let mut fallback = None;

    for path in paths {
        let Some(content) = fetch(path.to_string()).await? else {
            continue;
        };

        if codeowners::has_usable_rule(&content) {
            return Ok(Some(content));
        }

        if fallback.is_none() {
            fallback = Some(content);
        }
    }

    Ok(fallback)
}

async fn fetch_codeowners(
    octocrab: &octocrab::Octocrab,
    org: &str,
    repo: &str,
) -> Result<Option<String>, octocrab::Error> {
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
) -> Result<Option<String>, octocrab::Error> {
    let result = octocrab
        .repos(org, repo)
        .get_content()
        .path(path)
        .r#ref("HEAD")
        .send()
        .await;

    match result {
        Ok(content) => Ok(content
            .items
            .first()
            .and_then(|item| item.decoded_content())),
        Err(error) => match classify(&error) {
            Failure::NotFound => Ok(None),
            _ => Err(error),
        },
    }
}

async fn fetch_admin_teams(
    octocrab: &octocrab::Octocrab,
    org: &str,
    repo: &str,
) -> Result<Vec<String>, octocrab::Error> {
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
            Err(error) => match classify(&error) {
                Failure::NotFound | Failure::Forbidden => break,
                _ => return Err(error),
            },
        }
    }

    Ok(slugs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Fake fetcher over a fixed path -> content map, recording lookup order.
    fn fake<'a>(
        files: &'a [(&'a str, &'a str)],
        seen: &'a RefCell<Vec<String>>,
    ) -> impl FnMut(String) -> std::future::Ready<Result<Option<String>, &'static str>> + 'a {
        move |path: String| {
            seen.borrow_mut().push(path.clone());
            let hit = files
                .iter()
                .find(|(p, _)| *p == path)
                .map(|(_, c)| c.to_string());
            std::future::ready(Ok(hit))
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

            assert_eq!(got, Ok(want.map(str::to_string)), "{name}");
            assert_eq!(seen.borrow().as_slice(), *want_seen, "{name}");
        }
    }

    #[tokio::test]
    async fn stops_at_a_failed_fetch() {
        let seen = RefCell::new(Vec::new());
        let got = resolve_codeowners(CODEOWNERS_PATHS, |path: String| {
            seen.borrow_mut().push(path.clone());
            std::future::ready(match path.as_str() {
                ".github/CODEOWNERS" => Err("rate limited"),
                _ => Ok(Some("* @acme/team-b\n".to_string())),
            })
        })
        .await;

        assert_eq!(got, Err("rate limited"));
        assert_eq!(
            seen.borrow().as_slice(),
            &[".github/CODEOWNERS".to_string()]
        );
    }
}
