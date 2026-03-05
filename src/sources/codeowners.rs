/// Extract the top-level team from CODEOWNERS content.
///
/// Looks for the `* @org/team-name` rule and strips the `@org/` prefix.
/// If multiple teams are on the `*` rule, returns the first one.
pub fn extract_team(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Look for the wildcard rule: * @org/team
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.first() == Some(&"*") {
            // Find first @org/team pattern
            for part in &parts[1..] {
                if let Some(team) = parse_owner(part) {
                    return Some(team);
                }
            }
        }
    }
    None
}

fn parse_owner(owner: &str) -> Option<String> {
    let owner = owner.strip_prefix('@')?;
    // Handle @org/team-name format
    if let Some((_org, team)) = owner.split_once('/') {
        Some(team.to_string())
    } else {
        // Plain @username — not a team
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_wildcard() {
        let content = "* @acme/platform-team\n";
        assert_eq!(extract_team(content), Some("platform-team".to_string()));
    }

    #[test]
    fn with_comments() {
        let content = "# Top-level owners\n* @acme/core-team\n/docs @acme/docs-team\n";
        assert_eq!(extract_team(content), Some("core-team".to_string()));
    }

    #[test]
    fn multiple_teams_on_wildcard() {
        let content = "* @acme/team-a @acme/team-b\n";
        assert_eq!(extract_team(content), Some("team-a".to_string()));
    }

    #[test]
    fn no_wildcard_rule() {
        let content = "/src @acme/backend\n/web @acme/frontend\n";
        assert_eq!(extract_team(content), None);
    }

    #[test]
    fn username_not_team() {
        let content = "* @johndoe\n";
        assert_eq!(extract_team(content), None);
    }

    #[test]
    fn empty_file() {
        assert_eq!(extract_team(""), None);
    }

    #[test]
    fn only_comments() {
        assert_eq!(extract_team("# just comments\n# nothing else\n"), None);
    }
}
