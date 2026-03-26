use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TeamSuggestion {
    pub team: String,
    pub commits: usize,
    pub reviews: usize,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestionResult {
    pub suggestions: Vec<TeamSuggestion>,
    pub unresolved: Vec<String>,
    pub lookback_days: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestions_sort_by_total_activity_desc() {
        let mut suggestions = vec![
            TeamSuggestion {
                team: "low-team".to_string(),
                commits: 1,
                reviews: 0,
                members: vec!["alice".to_string()],
            },
            TeamSuggestion {
                team: "high-team".to_string(),
                commits: 5,
                reviews: 3,
                members: vec!["bob".to_string(), "carol".to_string()],
            },
            TeamSuggestion {
                team: "mid-team".to_string(),
                commits: 2,
                reviews: 2,
                members: vec!["dave".to_string()],
            },
        ];
        suggestions.sort_by(|a, b| {
            let a_total = a.commits + a.reviews;
            let b_total = b.commits + b.reviews;
            b_total
                .cmp(&a_total)
                .then(b.members.len().cmp(&a.members.len()))
        });
        assert_eq!(suggestions[0].team, "high-team");
        assert_eq!(suggestions[1].team, "mid-team");
        assert_eq!(suggestions[2].team, "low-team");
    }

    #[test]
    fn tiebreak_by_member_count() {
        let mut suggestions = vec![
            TeamSuggestion {
                team: "fewer-members".to_string(),
                commits: 3,
                reviews: 0,
                members: vec!["alice".to_string()],
            },
            TeamSuggestion {
                team: "more-members".to_string(),
                commits: 2,
                reviews: 1,
                members: vec!["bob".to_string(), "carol".to_string()],
            },
        ];
        suggestions.sort_by(|a, b| {
            let a_total = a.commits + a.reviews;
            let b_total = b.commits + b.reviews;
            b_total
                .cmp(&a_total)
                .then(b.members.len().cmp(&a.members.len()))
        });
        assert_eq!(suggestions[0].team, "more-members");
    }
}
