use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::cli::StatusFilter;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentStatus {
    Aligned,
    Mismatched,
    CatalogOnly,
    CodeownersOnly,
    Stale,
    Missing,
}

impl AlignmentStatus {
    pub fn matches_filter(&self, filters: &[StatusFilter]) -> bool {
        if filters.is_empty() {
            return true;
        }
        filters.iter().any(|f| match f {
            StatusFilter::Aligned => *self == AlignmentStatus::Aligned,
            StatusFilter::Mismatched => *self == AlignmentStatus::Mismatched,
            StatusFilter::CatalogOnly => *self == AlignmentStatus::CatalogOnly,
            StatusFilter::CodeownersOnly => *self == AlignmentStatus::CodeownersOnly,
            StatusFilter::Stale => *self == AlignmentStatus::Stale,
            StatusFilter::Missing => *self == AlignmentStatus::Missing,
        })
    }
}

impl std::fmt::Display for AlignmentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlignmentStatus::Aligned => write!(f, "aligned"),
            AlignmentStatus::Mismatched => write!(f, "mismatched"),
            AlignmentStatus::CatalogOnly => write!(f, "catalog-only"),
            AlignmentStatus::CodeownersOnly => write!(f, "codeowners-only"),
            AlignmentStatus::Stale => write!(f, "stale"),
            AlignmentStatus::Missing => write!(f, "missing"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoOwnership {
    pub repo_name: String,
    pub pushed_at: Option<DateTime<Utc>>,
    pub catalog_owner: Option<String>,
    pub codeowners_team: Option<String>,
    pub catalog_team_exists: Option<bool>,
    pub codeowners_team_exists: Option<bool>,
    pub alignment: AlignmentStatus,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditSummary {
    pub total: usize,
    pub aligned: usize,
    pub mismatched: usize,
    pub catalog_only: usize,
    pub codeowners_only: usize,
    pub stale: usize,
    pub missing: usize,
    pub repos: Vec<RepoOwnership>,
}

impl AuditSummary {
    pub fn from_repos(repos: Vec<RepoOwnership>) -> Self {
        let total = repos.len();
        let aligned = repos.iter().filter(|r| r.alignment == AlignmentStatus::Aligned).count();
        let mismatched = repos.iter().filter(|r| r.alignment == AlignmentStatus::Mismatched).count();
        let catalog_only = repos.iter().filter(|r| r.alignment == AlignmentStatus::CatalogOnly).count();
        let codeowners_only = repos.iter().filter(|r| r.alignment == AlignmentStatus::CodeownersOnly).count();
        let stale = repos.iter().filter(|r| r.alignment == AlignmentStatus::Stale).count();
        let missing = repos.iter().filter(|r| r.alignment == AlignmentStatus::Missing).count();

        AuditSummary {
            total,
            aligned,
            mismatched,
            catalog_only,
            codeowners_only,
            stale,
            missing,
            repos,
        }
    }

}
