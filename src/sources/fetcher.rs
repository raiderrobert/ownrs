use tokio::sync::Semaphore;
use std::sync::Arc;

use crate::cache::file_cache::FileCache;
use crate::github::client::GitHubClient;

/// Raw source files fetched for a single repo.
#[derive(Debug, Clone)]
pub struct RepoSources {
    pub repo_name: String,
    pub codeowners: Option<String>,
    pub catalog_info: Option<String>,
}

const CODEOWNERS_PATHS: &[&str] = &["CODEOWNERS", ".github/CODEOWNERS", "docs/CODEOWNERS"];
const CATALOG_PATH: &str = "catalog-info.yaml";

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
        let cache_key_co = format!("content_{org}_{name}_codeowners");
        let cache_key_cat = format!("content_{org}_{name}_catalog");

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

        // If both are cached, skip the network fetch
        if let (Some(co), Some(cat)) = (cached_co.clone(), cached_cat.clone()) {
            handles.push(tokio::spawn(async move {
                RepoSources {
                    repo_name: name,
                    codeowners: co,
                    catalog_info: cat,
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

            RepoSources {
                repo_name: name,
                codeowners,
                catalog_info,
            }
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        if let Ok(sources) = handle.await {
            let cache_key_co = format!("content_{org}_{}_codeowners", sources.repo_name);
            let cache_key_cat = format!("content_{org}_{}_catalog", sources.repo_name);
            let _ = cache.set(&cache_key_co, &sources.codeowners);
            let _ = cache.set(&cache_key_cat, &sources.catalog_info);

            results.push(sources);
        }
    }
    results
}

async fn fetch_codeowners(octocrab: &octocrab::Octocrab, org: &str, repo: &str) -> Option<String> {
    for path in CODEOWNERS_PATHS {
        if let Some(content) = fetch_file_content(octocrab, org, repo, path).await {
            return Some(content);
        }
    }
    None
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
        Ok(content) => {
            content.items.first().and_then(|item| {
                item.decoded_content()
            })
        }
        Err(_) => None,
    }
}
