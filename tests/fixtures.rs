use std::path::PathBuf;

use tempfile::TempDir;

pub const ORG: &str = "testorg";

#[derive(Debug, Clone)]
pub struct RepoRow {
    pub repo_name: String,
    pub catalog_owner: Option<String>,
    pub codeowners_teams: Vec<String>,
    pub admin_teams: Vec<String>,
    pub pushed_at: String,
}

pub fn parse_list(s: &str) -> Vec<String> {
    let s = s.trim();
    if s.is_empty() || s == "-" {
        return Vec::new();
    }
    s.split(',').map(|t| t.trim().to_string()).collect()
}

pub fn parse_optional(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() || s == "-" {
        None
    } else {
        Some(s.to_string())
    }
}

fn write_cache_file(cache_dir: &PathBuf, key: &str, json: &str) {
    let safe_key = key.replace('/', "__");
    let path = cache_dir.join(format!("{safe_key}.json"));
    std::fs::write(&path, json).unwrap_or_else(|e| {
        panic!("Failed to write cache file {}: {}", path.display(), e);
    });
}

fn build_catalog_yaml(owner: &str) -> String {
    format!(
        "apiVersion: backstage.io/v1alpha1\nkind: Component\nspec:\n  owner: group:{}\n",
        owner
    )
}

fn build_codeowners_content(teams: &[String]) -> String {
    let owners: Vec<String> = teams.iter().map(|t| format!("@{}/{}", ORG, t)).collect();
    format!("* {}\n", owners.join(" "))
}

/// Write cache fixture files for the given repos and valid teams.
///
/// Returns `(TempDir, PathBuf)` — the TempDir must be held alive for the
/// duration of the test, and the PathBuf is the cache directory path.
pub fn write_fixtures(
    repos: &[RepoRow],
    valid_teams: Option<&[String]>,
) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let cache_dir = temp_dir.path().to_path_buf();

    // Determine valid teams
    let valid_teams: Vec<String> = if let Some(explicit) = valid_teams {
        explicit.to_vec()
    } else {
        let mut teams = std::collections::HashSet::new();
        for repo in repos {
            if let Some(ref owner) = repo.catalog_owner {
                teams.insert(owner.clone());
            }
            for t in &repo.codeowners_teams {
                teams.insert(t.clone());
            }
            for t in &repo.admin_teams {
                teams.insert(t.clone());
            }
        }
        teams.into_iter().collect()
    };

    // Write teams cache
    let teams_json = serde_json::to_string(&valid_teams).unwrap();
    write_cache_file(&cache_dir, &format!("teams_{ORG}"), &teams_json);

    // Build repos list
    let repos_json: Vec<serde_json::Value> = repos
        .iter()
        .map(|r| {
            let pushed = if r.pushed_at.is_empty() || r.pushed_at == "-" {
                serde_json::Value::Null
            } else {
                let dt = if r.pushed_at.len() == 10 {
                    format!("{}T00:00:00Z", r.pushed_at)
                } else {
                    r.pushed_at.clone()
                };
                serde_json::Value::String(dt)
            };
            serde_json::json!({
                "name": r.repo_name,
                "pushed_at": pushed,
            })
        })
        .collect();
    write_cache_file(
        &cache_dir,
        &format!("repos_{ORG}"),
        &serde_json::to_string(&repos_json).unwrap(),
    );

    // Per-repo cache files
    for repo in repos {
        let name = &repo.repo_name;

        // CODEOWNERS
        let co_key = format!("content_{ORG}_{name}_codeowners");
        let co_value: serde_json::Value = if repo.codeowners_teams.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(build_codeowners_content(&repo.codeowners_teams))
        };
        write_cache_file(
            &cache_dir,
            &co_key,
            &serde_json::to_string(&co_value).unwrap(),
        );

        // catalog-info.yaml
        let cat_key = format!("content_{ORG}_{name}_catalog");
        let cat_value: serde_json::Value = match &repo.catalog_owner {
            Some(owner) => serde_json::Value::String(build_catalog_yaml(owner)),
            None => serde_json::Value::Null,
        };
        write_cache_file(
            &cache_dir,
            &cat_key,
            &serde_json::to_string(&cat_value).unwrap(),
        );

        // admin teams
        let admin_key = format!("admin_teams_{ORG}_{name}");
        write_cache_file(
            &cache_dir,
            &admin_key,
            &serde_json::to_string(&repo.admin_teams).unwrap(),
        );
    }

    (temp_dir, cache_dir)
}
