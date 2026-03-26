use crate::reconcile::types::RepoOwnership;

pub fn print_csv(repos: &[RepoOwnership]) {
    let mut wtr = csv::Writer::from_writer(std::io::stdout());

    wtr.write_record([
        "repo",
        "status",
        "catalog_owner",
        "codeowners_teams",
        "admin_teams",
        "catalog_team_exists",
        "pushed_at",
        "notes",
        "suggested_owners",
    ])
    .ok();

    for repo in repos {
        let codeowners_teams_str = repo.codeowners_teams.join(", ");
        let admin_teams_str = repo.admin_teams.join(", ");

        let suggested_str = repo
            .suggested_owners
            .as_ref()
            .map(|s| {
                s.suggestions
                    .iter()
                    .map(|t| t.team.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_default();

        wtr.write_record([
            &repo.repo_name,
            &repo.alignment.to_string(),
            repo.catalog_owner.as_deref().unwrap_or(""),
            &codeowners_teams_str,
            &admin_teams_str,
            &repo
                .catalog_team_exists
                .map(|b| b.to_string())
                .unwrap_or_default(),
            &repo.pushed_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
            &repo.notes.join("; "),
            &suggested_str,
        ])
        .ok();
    }

    wtr.flush().ok();
}
