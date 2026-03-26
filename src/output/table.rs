use comfy_table::{ContentArrangement, Table};

use crate::reconcile::types::{AuditSummary, RepoOwnership};

pub fn print_summary(summary: &AuditSummary) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Status", "Count", "%"]);

    let total = summary.total as f64;
    let pct = |n: usize| {
        if total == 0.0 {
            "0.0".to_string()
        } else {
            format!("{:.1}", n as f64 / total * 100.0)
        }
    };

    table.add_row(vec![
        "Aligned",
        &summary.aligned.to_string(),
        &pct(summary.aligned),
    ]);
    table.add_row(vec![
        "Mismatched",
        &summary.mismatched.to_string(),
        &pct(summary.mismatched),
    ]);
    table.add_row(vec![
        "Catalog Only",
        &summary.catalog_only.to_string(),
        &pct(summary.catalog_only),
    ]);
    table.add_row(vec![
        "Codeowners Only",
        &summary.codeowners_only.to_string(),
        &pct(summary.codeowners_only),
    ]);
    table.add_row(vec![
        "Admin Only",
        &summary.admin_only.to_string(),
        &pct(summary.admin_only),
    ]);
    table.add_row(vec![
        "Stale",
        &summary.stale.to_string(),
        &pct(summary.stale),
    ]);
    table.add_row(vec![
        "Missing",
        &summary.missing.to_string(),
        &pct(summary.missing),
    ]);
    table.add_row(vec!["Total", &summary.total.to_string(), ""]);

    println!("{table}");
}

pub fn print_detail(repos: &[RepoOwnership]) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        "Repo",
        "Status",
        "Catalog Owner",
        "CODEOWNERS Teams",
        "Admin Teams",
        "Last Push",
        "Notes",
    ]);

    for repo in repos {
        let pushed = repo
            .pushed_at
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "-".to_string());

        let codeowners_str = if repo.codeowners_teams.is_empty() {
            "-".to_string()
        } else {
            repo.codeowners_teams.join(", ")
        };

        let admin_str = if repo.admin_teams.is_empty() {
            "-".to_string()
        } else {
            repo.admin_teams.join(", ")
        };

        table.add_row(vec![
            &repo.repo_name,
            &repo.alignment.to_string(),
            repo.catalog_owner.as_deref().unwrap_or("-"),
            &codeowners_str,
            &admin_str,
            &pushed,
            &repo.notes.join("; "),
        ]);
    }

    println!("{table}");
}

pub fn print_single_repo(repo: &RepoOwnership) {
    println!("Repository: {}", repo.repo_name);
    println!("Status:     {}", repo.alignment);
    println!(
        "Catalog:    {}{}",
        repo.catalog_owner.as_deref().unwrap_or("(none)"),
        match repo.catalog_team_exists {
            Some(true) => " (team exists)",
            Some(false) => " (team NOT found)",
            None => "",
        }
    );

    if repo.codeowners_teams.is_empty() {
        println!("CODEOWNERS: (none)");
    } else {
        for (team, exists) in &repo.codeowners_teams_exist {
            let status = if *exists {
                "(team exists)"
            } else {
                "(team NOT found)"
            };
            println!("CODEOWNERS: {} {}", team, status);
        }
    }

    if repo.admin_teams.is_empty() {
        println!("Admin:      (none)");
    } else {
        println!("Admin:      {}", repo.admin_teams.join(", "));
    }

    if let Some(pushed) = repo.pushed_at {
        println!("Last Push:  {}", pushed.format("%Y-%m-%d %H:%M UTC"));
    }
    if !repo.notes.is_empty() {
        println!("Notes:      {}", repo.notes.join("; "));
    }

    if let Some(ref suggestion) = repo.suggested_owners {
        if suggestion.suggestions.is_empty() {
            println!(
                "\nSuggested:  No activity found in last {} days",
                suggestion.lookback_days
            );
        } else {
            println!(
                "\nSuggested owners (based on last {} days of activity):",
                suggestion.lookback_days
            );
            for s in &suggestion.suggestions {
                let commits_label = if s.commits == 1 { "commit" } else { "commits" };
                let reviews_label = if s.reviews == 1 { "review" } else { "reviews" };
                let members_str = s.members.join(", ");
                println!(
                    "  {:<16} {} {}, {} {} ({})",
                    s.team, s.commits, commits_label, s.reviews, reviews_label, members_str
                );
            }
        }
        if !suggestion.unresolved.is_empty() {
            println!(
                "\nUnresolved:        {}",
                suggestion.unresolved.join(", ")
            );
        }
    }
}
