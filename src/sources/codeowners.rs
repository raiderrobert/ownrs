/// Extract all top-level teams from CODEOWNERS content.
///
/// Looks for the `* @org/team-name` rule and strips the `@org/` prefix.
/// Returns all teams on the wildcard rule, deduplicated, preserving order.
pub fn extract_teams(content: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut teams = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.first() == Some(&"*") {
            for part in &parts[1..] {
                if let Some(team) = parse_owner(part) {
                    if seen.insert(team.clone()) {
                        teams.push(team);
                    }
                }
            }
            return teams;
        }
    }
    teams
}

fn parse_owner(owner: &str) -> Option<String> {
    let owner = owner.strip_prefix('@')?;
    if let Some((_org, team)) = owner.split_once('/') {
        Some(team.to_string())
    } else {
        None
    }
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
        assert_eq!(extract_teams("# just comments\n# nothing else\n"), Vec::<String>::new());
    }
}
