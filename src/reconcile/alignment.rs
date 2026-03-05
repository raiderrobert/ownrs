use std::collections::HashSet;

use super::types::{AlignmentStatus, RepoOwnership};
use chrono::{DateTime, Utc};

pub fn reconcile(
    repo_name: &str,
    pushed_at: Option<DateTime<Utc>>,
    catalog_owner: Option<&str>,
    codeowners_team: Option<&str>,
    valid_teams: &HashSet<String>,
) -> RepoOwnership {
    let catalog_team_exists = catalog_owner.map(|t| valid_teams.contains(t));
    let codeowners_team_exists = codeowners_team.map(|t| valid_teams.contains(t));

    let mut notes = Vec::new();

    let alignment = match (catalog_owner, codeowners_team) {
        (None, None) => {
            notes.push("No ownership metadata found".to_string());
            AlignmentStatus::Missing
        }
        (Some(cat), None) => {
            if catalog_team_exists == Some(false) {
                notes.push(format!("catalog-info.yaml references non-existent team: {cat}"));
                AlignmentStatus::Stale
            } else {
                notes.push("Only catalog-info.yaml has ownership".to_string());
                AlignmentStatus::CatalogOnly
            }
        }
        (None, Some(co)) => {
            if codeowners_team_exists == Some(false) {
                notes.push(format!("CODEOWNERS references non-existent team: {co}"));
                AlignmentStatus::Stale
            } else {
                notes.push("Only CODEOWNERS has ownership".to_string());
                AlignmentStatus::CodeownersOnly
            }
        }
        (Some(cat), Some(co)) => {
            let any_stale = catalog_team_exists == Some(false)
                || codeowners_team_exists == Some(false);

            if any_stale {
                if catalog_team_exists == Some(false) {
                    notes.push(format!("catalog-info.yaml references non-existent team: {cat}"));
                }
                if codeowners_team_exists == Some(false) {
                    notes.push(format!("CODEOWNERS references non-existent team: {co}"));
                }
                AlignmentStatus::Stale
            } else if normalize_team(cat) == normalize_team(co) {
                AlignmentStatus::Aligned
            } else {
                notes.push(format!(
                    "catalog-info.yaml says \"{cat}\", CODEOWNERS says \"{co}\""
                ));
                AlignmentStatus::Mismatched
            }
        }
    };

    RepoOwnership {
        repo_name: repo_name.to_string(),
        pushed_at,
        catalog_owner: catalog_owner.map(String::from),
        codeowners_team: codeowners_team.map(String::from),
        catalog_team_exists,
        codeowners_team_exists,
        alignment,
        notes,
    }
}

fn normalize_team(team: &str) -> String {
    team.to_lowercase()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn teams(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn aligned_when_both_match() {
        let result = reconcile("repo", None, Some("team-a"), Some("team-a"), &teams(&["team-a"]));
        assert_eq!(result.alignment, AlignmentStatus::Aligned);
        assert!(result.notes.is_empty());
    }

    #[test]
    fn aligned_case_insensitive() {
        let result = reconcile("repo", None, Some("Team-A"), Some("team-a"), &teams(&["team-a", "Team-A"]));
        assert_eq!(result.alignment, AlignmentStatus::Aligned);
    }

    #[test]
    fn mismatched_when_both_present_different() {
        let result = reconcile("repo", None, Some("team-a"), Some("team-b"), &teams(&["team-a", "team-b"]));
        assert_eq!(result.alignment, AlignmentStatus::Mismatched);
    }

    #[test]
    fn catalog_only() {
        let result = reconcile("repo", None, Some("team-a"), None, &teams(&["team-a"]));
        assert_eq!(result.alignment, AlignmentStatus::CatalogOnly);
    }

    #[test]
    fn codeowners_only() {
        let result = reconcile("repo", None, None, Some("team-a"), &teams(&["team-a"]));
        assert_eq!(result.alignment, AlignmentStatus::CodeownersOnly);
    }

    #[test]
    fn missing_when_neither() {
        let result = reconcile("repo", None, None, None, &teams(&["team-a"]));
        assert_eq!(result.alignment, AlignmentStatus::Missing);
    }

    #[test]
    fn stale_when_codeowners_team_gone() {
        let result = reconcile("repo", None, Some("team-a"), Some("team-gone"), &teams(&["team-a"]));
        assert_eq!(result.alignment, AlignmentStatus::Stale);
    }

    #[test]
    fn stale_when_catalog_team_gone() {
        let result = reconcile("repo", None, Some("team-gone"), Some("team-a"), &teams(&["team-a"]));
        assert_eq!(result.alignment, AlignmentStatus::Stale);
    }

    #[test]
    fn stale_catalog_only_nonexistent() {
        let result = reconcile("repo", None, Some("team-gone"), None, &teams(&["team-a"]));
        assert_eq!(result.alignment, AlignmentStatus::Stale);
    }

    #[test]
    fn stale_codeowners_only_nonexistent() {
        let result = reconcile("repo", None, None, Some("team-gone"), &teams(&["team-a"]));
        assert_eq!(result.alignment, AlignmentStatus::Stale);
    }
}
