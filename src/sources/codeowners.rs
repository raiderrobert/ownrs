/// What CODEOWNERS says about a repo, as far as reconciliation cares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeownersState {
    /// No CODEOWNERS file at any candidate path.
    Absent,
    /// A file exists but assigns no reviewers, so it owns nothing in practice.
    Unusable,
    /// A usable rule was found. `teams` is empty when the rule names only
    /// individual users, which is valid but gives nothing to reconcile against.
    Owned { teams: Vec<String> },
}

impl CodeownersState {
    /// Classify the selected CODEOWNERS file, or its absence.
    pub fn from_content(content: Option<&str>) -> Self {
        match content {
            None => Self::Absent,
            Some(c) if has_usable_rule(c) => Self::Owned {
                teams: extract_teams(c),
            },
            Some(_) => Self::Unusable,
        }
    }

    pub fn teams(&self) -> &[String] {
        match self {
            Self::Owned { teams } => teams,
            Self::Absent | Self::Unusable => &[],
        }
    }
}

/// Owners named on the `*` rule, split by kind.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct WildcardOwners {
    /// `@org/team` entries, with the `@org/` prefix stripped.
    pub teams: Vec<String>,
    /// `@username` entries, with the `@` stripped.
    pub users: Vec<String>,
}

impl WildcardOwners {
    pub fn is_empty(&self) -> bool {
        self.teams.is_empty() && self.users.is_empty()
    }
}

/// Parse the top-level `* ...` rule from CODEOWNERS content.
///
/// Returns `None` when the file has no `*` rule at all — an empty file, or one
/// that lost its path pattern and lists bare owners. Such a file assigns no
/// reviewers, which is what GitHub does with it too. A rule that is present but
/// names nobody yields an empty `WildcardOwners`.
///
/// Only the first `*` rule is considered, matching GitHub's last-match-wins
/// semantics for the top-level default.
pub fn parse_wildcard(content: &str) -> Option<WildcardOwners> {
    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.first() != Some(&"*") {
            continue;
        }

        let mut owners = WildcardOwners::default();
        let mut seen = std::collections::HashSet::new();

        for part in &parts[1..] {
            let Some(owner) = part.strip_prefix('@') else {
                continue;
            };
            if !seen.insert(owner.to_string()) {
                continue;
            }
            match owner.split_once('/') {
                Some((_org, team)) => owners.teams.push(team.to_string()),
                None => owners.users.push(owner.to_string()),
            }
        }

        return Some(owners);
    }

    None
}

/// True when GitHub would assign at least one reviewer from this file.
pub fn has_usable_rule(content: &str) -> bool {
    parse_wildcard(content).is_some_and(|o| !o.is_empty())
}

/// Extract all top-level teams from CODEOWNERS content.
///
/// Looks for the `* @org/team-name` rule and strips the `@org/` prefix.
/// Returns all teams on the wildcard rule, deduplicated, preserving order.
/// Individual users on the rule are not teams and are not returned.
pub fn extract_teams(content: &str) -> Vec<String> {
    parse_wildcard(content).map(|o| o.teams).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_wildcard() {
        let content = "* @acme/platform-team\n";
        assert_eq!(extract_teams(content), vec!["platform-team"]);
    }

    #[test]
    fn with_comments() {
        let content = "# Top-level owners\n* @acme/core-team\n/docs @acme/docs-team\n";
        assert_eq!(extract_teams(content), vec!["core-team"]);
    }

    #[test]
    fn multiple_teams_on_wildcard() {
        let content = "* @acme/team-a @acme/team-b\n";
        assert_eq!(extract_teams(content), vec!["team-a", "team-b"]);
    }

    #[test]
    fn mixed_users_and_teams() {
        let content = "* @acme/team-a @alice @acme/team-b\n";
        assert_eq!(extract_teams(content), vec!["team-a", "team-b"]);
    }

    #[test]
    fn duplicate_teams_deduplicated() {
        let content = "* @acme/team-a @acme/team-a @acme/team-b\n";
        assert_eq!(extract_teams(content), vec!["team-a", "team-b"]);
    }

    #[test]
    fn no_wildcard_rule() {
        let content = "/src @acme/backend\n/web @acme/frontend\n";
        assert_eq!(extract_teams(content), Vec::<String>::new());
    }

    #[test]
    fn username_not_team() {
        let content = "* @johndoe\n";
        assert_eq!(extract_teams(content), Vec::<String>::new());
    }

    #[test]
    fn empty_file() {
        assert_eq!(extract_teams(""), Vec::<String>::new());
    }

    #[test]
    fn only_comments() {
        assert_eq!(
            extract_teams("# just comments\n# nothing else\n"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn wildcard_splits_teams_from_users() {
        let owners = parse_wildcard("* @acme/team-a @alice @acme/team-b @bob\n").unwrap();
        assert_eq!(owners.teams, vec!["team-a", "team-b"]);
        assert_eq!(owners.users, vec!["alice", "bob"]);
    }

    #[test]
    fn usable_when_rule_names_only_users() {
        // Valid CODEOWNERS — GitHub assigns these reviewers — it just has no teams.
        let content = "* @alice\n";
        assert!(has_usable_rule(content));
        assert_eq!(extract_teams(content), Vec::<String>::new());
    }

    #[test]
    fn unusable_when_no_wildcard_rule() {
        assert_eq!(parse_wildcard("@acme/team-a @acme/team-b\n"), None);
        assert!(!has_usable_rule("@acme/team-a @acme/team-b\n"));
    }

    #[test]
    fn unusable_when_file_is_empty_or_comments_only() {
        assert_eq!(parse_wildcard(""), None);
        assert!(!has_usable_rule(""));
        assert!(!has_usable_rule("# managed by terraform\n"));
    }

    #[test]
    fn unusable_when_wildcard_names_nobody() {
        let owners = parse_wildcard("* \n").unwrap();
        assert!(owners.is_empty());
        assert!(!has_usable_rule("* \n"));
    }

    #[test]
    fn state_absent_when_no_file() {
        assert_eq!(CodeownersState::from_content(None), CodeownersState::Absent);
    }

    #[test]
    fn state_unusable_for_a_file_that_assigns_nobody() {
        assert_eq!(
            CodeownersState::from_content(Some("@acme/team-a\n")),
            CodeownersState::Unusable
        );
        assert_eq!(
            CodeownersState::from_content(Some("")),
            CodeownersState::Unusable
        );
    }

    #[test]
    fn state_owned_with_no_teams_for_a_user_only_rule() {
        let state = CodeownersState::from_content(Some("* @alice\n"));
        assert_eq!(state, CodeownersState::Owned { teams: vec![] });
        assert!(state.teams().is_empty());
    }

    #[test]
    fn state_owned_carries_teams() {
        let state = CodeownersState::from_content(Some("* @acme/team-a\n"));
        assert_eq!(state.teams(), ["team-a".to_string()]);
    }

    #[test]
    fn path_scoped_rules_do_not_make_a_file_usable() {
        // GitHub honors these, but they are not a top-level default owner.
        assert!(!has_usable_rule("/src @acme/backend\n"));
    }
}
