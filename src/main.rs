mod cache;
mod cli;
mod config;
mod github;
mod output;
mod reconcile;
mod sources;

use std::process;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

use cache::file_cache::FileCache;
use cli::{OutputFormat, SortOrder};
use config::{Config, Scope};
use github::client::GitHubClient;
use github::repos::list_repos;
use github::teams::fetch_team_slugs;
use reconcile::alignment::reconcile;
use reconcile::types::AuditSummary;
use sources::fetcher::fetch_all;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e:#}");
        process::exit(2);
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    let config = Config::from_cli(cli)?;
    let client = GitHubClient::new(&config.token)?;
    let cache = FileCache::new(config.cache_dir.clone(), config.cache_ttl)?;

    match config.scope {
        Scope::Org {
            ref org,
            limit,
            ref sort,
            ref team_filter,
            ref status_filter,
            ref format,
            detail,
            strict,
        } => run_org(&client, &cache, &config, org, limit, sort, team_filter, status_filter, format, detail, strict).await,
        Scope::Repo {
            ref org,
            ref repo,
            ref status_filter,
            ref format,
            strict,
        } => run_repo(&client, &cache, &config, org, repo, status_filter, format, strict).await,
    }
}

async fn run_org(
    client: &GitHubClient,
    cache: &FileCache,
    config: &Config,
    org: &str,
    limit: Option<usize>,
    sort: &SortOrder,
    team_filter: &[String],
    status_filter: &[cli::StatusFilter],
    format: &OutputFormat,
    detail: bool,
    strict: bool,
) -> anyhow::Result<()> {
    // Fetch teams
    let sp = ProgressBar::new_spinner();
    sp.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    sp.set_message("Fetching teams...");
    sp.enable_steady_tick(std::time::Duration::from_millis(100));
    let valid_teams = fetch_team_slugs(client, org, cache, config.refresh).await?;
    sp.finish_with_message(format!("Fetched {} teams", valid_teams.len()));

    // Fetch repos
    let sp = ProgressBar::new_spinner();
    sp.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    sp.set_message("Fetching repos...");
    sp.enable_steady_tick(std::time::Duration::from_millis(100));
    let mut repos = list_repos(client, org, cache, config.refresh, |count| {
        sp.set_message(format!("Fetching repos... {count} so far"));
    }).await?;
    sp.finish_with_message(format!("Fetched {} repos", repos.len()));

    // Sort
    match sort {
        SortOrder::Stale => repos.sort_by(|a, b| a.pushed_at.cmp(&b.pushed_at)),
        SortOrder::Active => repos.sort_by(|a, b| b.pushed_at.cmp(&a.pushed_at)),
        SortOrder::Name => repos.sort_by(|a, b| a.name.cmp(&b.name)),
    }

    // Limit
    if let Some(n) = limit {
        repos.truncate(n);
    }

    let repo_names: Vec<String> = repos.iter().map(|r| r.name.clone()).collect();

    // Fetch source files
    let pb = ProgressBar::new(repo_names.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40}] {pos}/{len}")
            .unwrap()
            .progress_chars("=> "),
    );
    pb.set_message("Fetching source files");

    let all_sources = fetch_all(client, org, &repo_names, cache, config.refresh).await;
    pb.finish_and_clear();

    // Reconcile
    let mut ownership_results = Vec::new();
    for source in &all_sources {
        let repo_info = repos.iter().find(|r| r.name == source.repo_name);
        let pushed_at = repo_info.and_then(|r| r.pushed_at);

        let catalog_owner = source.catalog_info.as_deref()
            .and_then(sources::catalog::extract_owner);
        let codeowners_teams = source.codeowners.as_deref()
            .map(sources::codeowners::extract_teams)
            .unwrap_or_default();

        let result = reconcile(
            &source.repo_name,
            pushed_at,
            catalog_owner.as_deref(),
            &codeowners_teams,
            &source.admin_teams,
            &valid_teams,
            strict,
        );

        ownership_results.push(result);
    }

    // Apply team filter
    if !team_filter.is_empty() {
        ownership_results.retain(|r| {
            let cat_match = r.catalog_owner.as_ref()
                .is_some_and(|o| team_filter.iter().any(|t| o.eq_ignore_ascii_case(t)));
            let co_match = r.codeowners_teams.iter()
                .any(|o| team_filter.iter().any(|t| o.eq_ignore_ascii_case(t)));
            let admin_match = r.admin_teams.iter()
                .any(|o| team_filter.iter().any(|t| o.eq_ignore_ascii_case(t)));
            cat_match || co_match || admin_match
        });
    }

    // Apply status filter
    if !status_filter.is_empty() {
        ownership_results.retain(|r| r.alignment.matches_filter(status_filter));
    }

    let summary = AuditSummary::from_repos(ownership_results);

    match format {
        OutputFormat::Json => output::json::print_json(&summary),
        OutputFormat::Csv => output::csv::print_csv(&summary.repos),
        OutputFormat::Table => {
            output::table::print_summary(&summary);
            if detail {
                println!();
                output::table::print_detail(&summary.repos);
            }
        }
    }

    // Exit code
    if !status_filter.is_empty() && !summary.repos.is_empty() {
        process::exit(1);
    }

    Ok(())
}

async fn run_repo(
    client: &GitHubClient,
    cache: &FileCache,
    config: &Config,
    org: &str,
    repo: &str,
    status_filter: &[cli::StatusFilter],
    format: &OutputFormat,
    strict: bool,
) -> anyhow::Result<()> {
    let sp = ProgressBar::new_spinner();
    sp.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}").unwrap());
    sp.set_message("Fetching teams...");
    sp.enable_steady_tick(std::time::Duration::from_millis(100));
    let valid_teams = fetch_team_slugs(client, org, cache, config.refresh).await?;
    sp.finish_with_message(format!("Fetched {} teams", valid_teams.len()));

    let repo_names = vec![repo.to_string()];
    let sources = fetch_all(client, org, &repo_names, cache, config.refresh).await;

    let source = sources.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!("Failed to fetch repo sources"))?;

    let catalog_owner = source.catalog_info.as_deref()
        .and_then(sources::catalog::extract_owner);
    let codeowners_teams = source.codeowners.as_deref()
        .map(sources::codeowners::extract_teams)
        .unwrap_or_default();

    let result = reconcile(
        &source.repo_name,
        None,
        catalog_owner.as_deref(),
        &codeowners_teams,
        &source.admin_teams,
        &valid_teams,
        strict,
    );

    if !status_filter.is_empty() && !result.alignment.matches_filter(status_filter) {
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let summary = AuditSummary::from_repos(vec![result]);
            output::json::print_json(&summary);
        }
        OutputFormat::Table => output::table::print_single_repo(&result),
        OutputFormat::Csv => output::csv::print_csv(&[result]),
    }

    if !status_filter.is_empty() {
        process::exit(1);
    }

    Ok(())
}

