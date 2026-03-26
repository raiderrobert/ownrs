use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "ownrs",
    version,
    about = "Three-way ownership reconciliation CLI"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Force re-fetch cached data
    #[arg(long, global = true)]
    pub refresh: bool,

    /// Cache directory
    #[arg(long, global = true, default_value = None)]
    pub cache_dir: Option<String>,

    /// Cache TTL in seconds
    #[arg(long, global = true, default_value_t = 86400)]
    pub cache_ttl: u64,

    /// GitHub token (defaults to GITHUB_TOKEN env var)
    #[arg(long, global = true, env = "GITHUB_TOKEN", hide_env_values = true)]
    pub token: Option<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Audit repos across a GitHub org
    Org {
        /// GitHub organization name
        org: String,

        /// Audit only the first N repos
        #[arg(long)]
        limit: Option<usize>,

        /// Sort order: stale (default), active, name
        #[arg(long, default_value = "stale")]
        sort: SortOrder,

        /// Filter to repos referencing this team (comma-separated)
        #[arg(long, value_delimiter = ',')]
        team: Vec<String>,

        /// Filter by alignment status (comma-separated)
        #[arg(long, value_delimiter = ',')]
        status: Vec<StatusFilter>,

        /// Output format: table (default), csv, json
        #[arg(long, default_value = "table")]
        format: OutputFormat,

        /// Show per-repo breakdown
        #[arg(long)]
        detail: bool,

        /// Require exact team set match across all sources (default: intersection)
        #[arg(long)]
        strict: bool,
    },

    /// Audit a single repo
    Repo {
        /// org/repo (auto-detected from git remote if omitted)
        repo: Option<String>,

        /// Filter by alignment status (comma-separated)
        #[arg(long, value_delimiter = ',')]
        status: Vec<StatusFilter>,

        /// Output format: table (default), json
        #[arg(long, default_value = "table")]
        format: OutputFormat,

        /// Require exact team set match across all sources (default: intersection)
        #[arg(long)]
        strict: bool,
    },
}

#[derive(Clone, ValueEnum)]
pub enum SortOrder {
    Stale,
    Active,
    Name,
}

#[derive(Clone, ValueEnum, PartialEq, Eq)]
pub enum StatusFilter {
    Aligned,
    Mismatched,
    Stale,
    Missing,
    CatalogOnly,
    CodeownersOnly,
    AdminOnly,
}

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Table,
    Csv,
    Json,
}
