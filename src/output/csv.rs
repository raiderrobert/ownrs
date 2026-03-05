use crate::reconcile::types::RepoOwnership;

pub fn print_csv(repos: &[RepoOwnership]) {
    let mut wtr = csv::Writer::from_writer(std::io::stdout());

    wtr.write_record([
        "repo",
        "status",
        "catalog_owner",
        "codeowners_team",
        "catalog_team_exists",
        "codeowners_team_exists",
        "pushed_at",
        "notes",
    ])
    .ok();

    for repo in repos {
        wtr.write_record([
            &repo.repo_name,
            &repo.alignment.to_string(),
            repo.catalog_owner.as_deref().unwrap_or(""),
            repo.codeowners_team.as_deref().unwrap_or(""),
            &repo.catalog_team_exists.map(|b| b.to_string()).unwrap_or_default(),
            &repo.codeowners_team_exists.map(|b| b.to_string()).unwrap_or_default(),
            &repo.pushed_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
            &repo.notes.join("; "),
        ])
        .ok();
    }

    wtr.flush().ok();
}
