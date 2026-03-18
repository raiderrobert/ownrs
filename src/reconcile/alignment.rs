use std::collections::HashSet;

use super::types::{AlignmentStatus, RepoOwnership};
use chrono::{DateTime, Utc};

pub fn reconcile(
    repo_name: &str,
    pushed_at: Option<DateTime<Utc>>,
    catalog_owner: Option<&str>,
    codeowners_teams: &[String],
    admin_teams: &[String],
    valid_teams: &HashSet<String>,
    strict: bool,
) -> RepoOwnership {
    let mut notes = Vec::new();

    // Phase 1 — Stale detection: check all referenced teams against valid_teams
    let catalog_team_exists = catalog_owner.map(|t| valid_teams.contains(t));

    let codeowners_teams_exist: Vec<(String, bool)> = codeowners_teams
        .iter()
        .map(|t| (t.clone(), valid_teams.contains(t.as_str())))
        .collect();

    let mut any_stale = false;

    if let Some(cat) = catalog_owner {
        if catalog_team_exists == Some(false) {
            notes.push(format!(
                "catalog-info.yaml references non-existent team: {cat}"
            ));
            any_stale = true;
        }
    }

    for (team, exists) in &codeowners_teams_exist {
        if !exists {
            notes.push(format!(
                "CODEOWNERS references non-existent team: {team}"
            ));
            any_stale = true;
        }
    }

    for team in admin_teams {
        if !valid_teams.contains(team.as_str()) {
            notes.push(format!(
                "Admin team does not exist: {team}"
            ));
            any_stale = true;
        }
    }

    if any_stale {
        return RepoOwnership {
            repo_name: repo_name.to_string(),
            pushed_at,
            catalog_owner: catalog_owner.map(String::from),
            codeowners_teams: codeowners_teams.to_vec(),
            catalog_team_exists,
            codeowners_teams_exist,
            admin_teams: admin_teams.to_vec(),
            alignment: AlignmentStatus::Stale,
            notes,
        };
    }

    // Phase 2 — Alignment: count present sources
    let has_catalog = catalog_owner.is_some();
    let has_codeowners = !codeowners_teams.is_empty();
    let has_admin = !admin_teams.is_empty();

    let source_count =
        has_catalog as usize + has_codeowners as usize + has_admin as usize;

    let alignment = match source_count {
        0 => {
            notes.push("No ownership metadata found".to_string());
            AlignmentStatus::Missing
        }
        1 => {
            if has_catalog {
                notes.push("Only catalog-info.yaml has ownership".to_string());
                AlignmentStatus::CatalogOnly
            } else if has_codeowners {
                notes.push("Only CODEOWNERS has ownership".to_string());
                AlignmentStatus::CodeownersOnly
            } else {
                notes.push("Only admin team membership has ownership".to_string());
                AlignmentStatus::AdminOnly
            }
        }
        _ => {
            // Build normalized HashSet per source
            let catalog_set: HashSet<String> = catalog_owner
                .iter()
                .map(|t| normalize_team(t))
                .collect();

            let codeowners_set: HashSet<String> = codeowners_teams
                .iter()
                .map(|t| normalize_team(t))
                .collect();

            let admin_set: HashSet<String> = admin_teams
                .iter()
                .map(|t| normalize_team(t))
                .collect();

            // Collect only the sets that are present
            let mut present_sets: Vec<&HashSet<String>> = Vec::new();
            if has_catalog {
                present_sets.push(&catalog_set);
            }
            if has_codeowners {
                present_sets.push(&codeowners_set);
            }
            if has_admin {
                present_sets.push(&admin_set);
            }

            if strict {
                // Strict mode: all present sets must be identical
                let first = present_sets[0];
                if present_sets.iter().all(|s| *s == first) {
                    AlignmentStatus::Aligned
                } else {
                    notes.push(
                        "Sources have different team sets (strict mode)".to_string(),
                    );
                    AlignmentStatus::Mismatched
                }
            } else {
                // Intersection mode: global intersection must be non-empty
                let mut intersection = present_sets[0].clone();
                for set in &present_sets[1..] {
                    intersection = intersection
                        .intersection(set)
                        .cloned()
                        .collect();
                }

                if !intersection.is_empty() {
                    AlignmentStatus::Aligned
                } else {
                    notes.push(
                        "No common team across all ownership sources".to_string(),
                    );
                    AlignmentStatus::Mismatched
                }
            }
        }
    };

    RepoOwnership {
        repo_name: repo_name.to_string(),
        pushed_at,
        catalog_owner: catalog_owner.map(String::from),
        codeowners_teams: codeowners_teams.to_vec(),
        catalog_team_exists,
        codeowners_teams_exist,
        admin_teams: admin_teams.to_vec(),
        alignment,
        notes,
    }
}

fn normalize_team(team: &str) -> String {
    team.to_lowercase().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn teams(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn sv(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn all_three_agree() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-a"]), &sv(&["team-a"]), &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Aligned);
    }

    #[test]
    fn two_of_three_overlap() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-a", "team-b"]), &sv(&["team-a", "team-c"]), &teams(&["team-a", "team-b", "team-c"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Aligned);
    }

    #[test]
    fn no_overlap_across_three() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-b"]), &sv(&["team-c"]), &teams(&["team-a", "team-b", "team-c"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Mismatched);
    }

    #[test]
    fn two_sources_catalog_codeowners_aligned() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-a"]), &[], &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Aligned);
    }

    #[test]
    fn two_sources_catalog_codeowners_mismatched() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-b"]), &[], &teams(&["team-a", "team-b"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Mismatched);
    }

    #[test]
    fn catalog_only() {
        let result = reconcile("repo", None, Some("team-a"), &[], &[], &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::CatalogOnly);
    }

    #[test]
    fn codeowners_only() {
        let result = reconcile("repo", None, None, &sv(&["team-a"]), &[], &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::CodeownersOnly);
    }

    #[test]
    fn admin_only() {
        let result = reconcile("repo", None, None, &[], &sv(&["team-a"]), &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::AdminOnly);
    }

    #[test]
    fn missing_when_none() {
        let result = reconcile("repo", None, None, &[], &[], &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Missing);
    }

    #[test]
    fn stale_catalog_team_gone() {
        let result = reconcile("repo", None, Some("team-gone"), &sv(&["team-a"]), &sv(&["team-a"]), &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Stale);
    }

    #[test]
    fn stale_codeowners_team_gone() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-gone"]), &[], &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Stale);
    }

    #[test]
    fn stale_admin_team_gone() {
        let result = reconcile("repo", None, None, &[], &sv(&["team-gone"]), &teams(&["team-a"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Stale);
    }

    #[test]
    fn case_insensitive_alignment() {
        let result = reconcile("repo", None, Some("Team-A"), &sv(&["team-a"]), &[], &teams(&["team-a", "Team-A"]), false);
        assert_eq!(result.alignment, AlignmentStatus::Aligned);
    }

    #[test]
    fn strict_all_identical() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-a"]), &sv(&["team-a"]), &teams(&["team-a"]), true);
        assert_eq!(result.alignment, AlignmentStatus::Aligned);
    }

    #[test]
    fn strict_superset_mismatched() {
        let result = reconcile("repo", None, Some("team-a"), &sv(&["team-a", "team-b"]), &sv(&["team-a"]), &teams(&["team-a", "team-b"]), true);
        assert_eq!(result.alignment, AlignmentStatus::Mismatched);
    }

    #[test]
    fn strict_single_source() {
        let result = reconcile("repo", None, Some("team-a"), &[], &[], &teams(&["team-a"]), true);
        assert_eq!(result.alignment, AlignmentStatus::CatalogOnly);
    }
}
