use std::path::PathBuf;

use crate::cli::{Cli, Command, OutputFormat, SortOrder, StatusFilter};

pub struct Config {
    pub scope: Scope,
    pub token: String,
    pub refresh: bool,
    pub cache_dir: PathBuf,
    pub cache_ttl: u64,
}

pub enum Scope {
    Org {
        org: String,
        limit: Option<usize>,
        sort: SortOrder,
        team_filter: Vec<String>,
        status_filter: Vec<StatusFilter>,
        format: OutputFormat,
        detail: bool,
        strict: bool,
    },
    Repo {
        org: String,
        repo: String,
        status_filter: Vec<StatusFilter>,
        format: OutputFormat,
        strict: bool,
    },
}

impl Config {
    pub fn from_cli(cli: Cli) -> anyhow::Result<Self> {
        let token = match cli.token {
            Some(t) => t,
            None => token_from_gh_cli()?,
        };

        let cache_dir = match cli.cache_dir {
            Some(dir) => PathBuf::from(dir),
            None => default_cache_dir()?,
        };

        let scope = match cli.command {
            Command::Org {
                org,
                limit,
                sort,
                team,
                status,
                format,
                detail,
                strict,
            } => Scope::Org {
                org,
                limit,
                sort,
                team_filter: team,
                status_filter: status,
                format,
                detail,
                strict,
            },
            Command::Repo {
                repo,
                status,
                format,
                strict,
            } => {
                let (org, repo_name) = parse_repo_arg(repo)?;
                Scope::Repo {
                    org,
                    repo: repo_name,
                    status_filter: status,
                    format,
                    strict,
                }
            }
        };

        Ok(Config {
            scope,
            token,
            refresh: cli.refresh,
            cache_dir,
            cache_ttl: cli.cache_ttl,
        })
    }
}

fn token_from_gh_cli() -> anyhow::Result<String> {
    let output = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let token = String::from_utf8(out.stdout)?.trim().to_string();
            if token.is_empty() {
                anyhow::bail!(
                    "GitHub token required. Run `gh auth login` or set GITHUB_TOKEN"
                );
            }
            Ok(token)
        }
        _ => anyhow::bail!(
            "GitHub token required. Run `gh auth login` or set GITHUB_TOKEN"
        ),
    }
}

fn default_cache_dir() -> anyhow::Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("", "", "ownrs")
        .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?;
    Ok(proj_dirs.cache_dir().to_path_buf())
}

fn parse_repo_arg(repo: Option<String>) -> anyhow::Result<(String, String)> {
    match repo {
        Some(slug) => {
            let parts: Vec<&str> = slug.splitn(2, '/').collect();
            if parts.len() != 2 {
                anyhow::bail!("Repo must be in org/repo format, got: {slug}");
            }
            Ok((parts[0].to_string(), parts[1].to_string()))
        }
        None => detect_from_git_remote(),
    }
}

fn detect_from_git_remote() -> anyhow::Result<(String, String)> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("No repo specified and could not detect from git remote");
    }

    let url = String::from_utf8(output.stdout)?.trim().to_string();

    // Handle SSH: git@github.com:org/repo.git
    if let Some(path) = url.strip_prefix("git@github.com:") {
        let path = path.strip_suffix(".git").unwrap_or(path);
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    // Handle HTTPS: https://github.com/org/repo.git
    if let Some(path) = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
    {
        let path = path.strip_suffix(".git").unwrap_or(path);
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    anyhow::bail!("Could not parse org/repo from git remote: {url}")
}
