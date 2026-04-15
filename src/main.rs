use std::process;

use chrono::Utc;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

use ownrs::cache::file_cache::FileCache;
use ownrs::cli::{self, OutputFormat, StatusFilter, SuggestMode};
use ownrs::config::{Config, Scope};
use ownrs::github::client::GitHubClient;
use ownrs::github::members::fetch_team_members;
use ownrs::github::repos::list_repos;
use ownrs::github::teams::fetch_team_slugs;
use ownrs::output;
use ownrs::reconcile::alignment::reconcile;
use ownrs::reconcile::types::{AlignmentStatus, AuditSummary};
use ownrs::sources;
use ownrs::sources::fetcher::fetch_all;
use ownrs::suggest::{fetch_commit_authors, fetch_pr_reviewers, score_teams};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e:#}");
        process::exit(2);
    }
}

async fn run() -> anyhow::Result<()> {
    let cli_args = cli::Cli::parse();
    let config = Config::from_cli(cli_args)?;
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
            summary,
            wide,
            strict,
        } => {
            run_org(
                &client,
                &cache,
                &config,
                org,
                limit,
                sort,
                team_filter,
                status_filter,
                format,
                summary,
                wide,
                strict,
            )
            .await
        }
        Scope::Repo {
            ref org,
            ref repo,
            ref status_filter,
            ref format,
            strict,
            ref suggest,
        } => {
            run_repo(
                &client,
                &cache,
                &config,
                org,
                repo,
                status_filter,
                format,
                strict,
                suggest,
                config.lookback_days,
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_org(
    client: &GitHubClient,
    cache: &FileCache,
    config: &Config,
    org: &str,
    limit: Option<usize>,
    sort: &[String],
    team_filter: &[String],
    status_filter: &[StatusFilter],
    format: &OutputFormat,
    summary: bool,
    wide: bool,
    strict: bool,
) -> anyhow::Result<()> {
    // Fetch teams
    let sp = ProgressBar::new_spinner();
    sp.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap(),
    );
    sp.set_message("Fetching teams...");
    sp.enable_steady_tick(std::time::Duration::from_millis(100));
    let valid_teams = fetch_team_slugs(client, org, cache, config.refresh).await?;
    sp.finish_with_message(format!("Fetched {} teams", valid_teams.len()));

    // Fetch repos
    let sp = ProgressBar::new_spinner();
    sp.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap(),
    );
    sp.set_message("Fetching repos...");
    sp.enable_steady_tick(std::time::Duration::from_millis(100));
    let mut repos = list_repos(client, org, cache, config.refresh, |count| {
        sp.set_message(format!("Fetching repos... {count} so far"));
    })
    .await?;
    sp.finish_with_message(format!("Fetched {} repos", repos.len()));

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

        let catalog_owner = source
            .catalog_info
            .as_deref()
            .and_then(sources::catalog::extract_owner);
        let codeowners_teams = source
            .codeowners
            .as_deref()
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
            let cat_match = r
                .catalog_owner
                .as_ref()
                .is_some_and(|o| team_filter.iter().any(|t| o.eq_ignore_ascii_case(t)));
            let co_match = r
                .codeowners_teams
                .iter()
                .any(|o| team_filter.iter().any(|t| o.eq_ignore_ascii_case(t)));
            let admin_match = r
                .admin_teams
                .iter()
                .any(|o| team_filter.iter().any(|t| o.eq_ignore_ascii_case(t)));
            cat_match || co_match || admin_match
        });
    }

    // Apply status filter
    if !status_filter.is_empty() {
        ownership_results.retain(|r| r.alignment.matches_filter(status_filter));
    }

    let audit = AuditSummary::from_repos(ownership_results);

    match format {
        OutputFormat::Json => output::json::print_json(&audit),
        OutputFormat::Csv => output::csv::print_csv(&audit.repos),
        OutputFormat::Names => {
            print!("{}", output::table::render_names(&audit.repos));
        }
        OutputFormat::Table => {
            if summary {
                println!("{}", output::table::render_summary(&audit));
            }
            let team_label = if team_filter.is_empty() {
                None
            } else {
                Some(team_filter.join(","))
            };
            let opts = output::table::TableOptions {
                wide,
                sort_columns: sort.to_vec(),
                team_filter: team_label,
            };
            print!("{}", output::table::render_table(&audit.repos, &opts));
        }
    }

    // Exit code
    if !status_filter.is_empty() && !audit.repos.is_empty() {
        process::exit(1);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_repo(
    client: &GitHubClient,
    cache: &FileCache,
    config: &Config,
    org: &str,
    repo: &str,
    status_filter: &[StatusFilter],
    format: &OutputFormat,
    strict: bool,
    suggest: &[SuggestMode],
    lookback_days: u64,
) -> anyhow::Result<()> {
    let sp = ProgressBar::new_spinner();
    sp.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap(),
    );
    sp.set_message("Fetching teams...");
    sp.enable_steady_tick(std::time::Duration::from_millis(100));
    let valid_teams = fetch_team_slugs(client, org, cache, config.refresh).await?;
    sp.finish_with_message(format!("Fetched {} teams", valid_teams.len()));

    let repo_names = vec![repo.to_string()];
    let sources = fetch_all(client, org, &repo_names, cache, config.refresh).await;

    let source = sources
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to fetch repo sources"))?;

    let catalog_owner = source
        .catalog_info
        .as_deref()
        .and_then(sources::catalog::extract_owner);
    let codeowners_teams = source
        .codeowners
        .as_deref()
        .map(sources::codeowners::extract_teams)
        .unwrap_or_default();

    let mut result = reconcile(
        &source.repo_name,
        None,
        catalog_owner.as_deref(),
        &codeowners_teams,
        &source.admin_teams,
        &valid_teams,
        strict,
    );

    let should_suggest = if suggest.is_empty() {
        // Default: auto-trigger for missing and stale
        result.alignment == AlignmentStatus::Missing || result.alignment == AlignmentStatus::Stale
    } else {
        suggest.iter().any(|mode| match mode {
            SuggestMode::Missing => result.alignment == AlignmentStatus::Missing,
            SuggestMode::Stale => result.alignment == AlignmentStatus::Stale,
            SuggestMode::Mismatched => result.alignment == AlignmentStatus::Mismatched,
            SuggestMode::Partial => matches!(
                result.alignment,
                AlignmentStatus::CatalogOnly
                    | AlignmentStatus::CodeownersOnly
                    | AlignmentStatus::AdminOnly
            ),
        })
    };

    if should_suggest {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner} {msg}")
                .unwrap(),
        );
        sp.set_message("Analyzing activity...");
        sp.enable_steady_tick(std::time::Duration::from_millis(100));

        let since = Utc::now() - chrono::Duration::days(lookback_days as i64);

        let team_slugs: Vec<String> = valid_teams.iter().cloned().collect();
        let team_members =
            fetch_team_members(client, org, &team_slugs, cache, config.refresh).await?;

        let commit_authors =
            fetch_commit_authors(client, org, repo, &since, cache, config.refresh).await?;
        let pr_reviewers =
            fetch_pr_reviewers(client, org, repo, &since, cache, config.refresh).await?;

        let suggestion = score_teams(
            &team_members,
            &commit_authors,
            &pr_reviewers,
            lookback_days,
            config.max_team_size,
            &config.exclude_team,
        );
        sp.finish_and_clear();

        result.suggested_owners = Some(suggestion);
    }

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
        OutputFormat::Names => {
            println!("{}", result.repo_name);
        }
    }

    if !status_filter.is_empty() {
        process::exit(1);
    }

    Ok(())
}
