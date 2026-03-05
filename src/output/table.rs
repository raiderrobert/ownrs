use comfy_table::{ContentArrangement, Table};

use crate::reconcile::types::{AuditSummary, RepoOwnership};

pub fn print_summary(summary: &AuditSummary) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Status", "Count", "%"]);

    let total = summary.total as f64;
    let pct = |n: usize| {
        if total == 0.0 { "0.0".to_string() } else { format!("{:.1}", n as f64 / total * 100.0) }
    };

    table.add_row(vec!["Aligned", &summary.aligned.to_string(), &pct(summary.aligned)]);
    table.add_row(vec!["Mismatched", &summary.mismatched.to_string(), &pct(summary.mismatched)]);
    table.add_row(vec!["Catalog Only", &summary.catalog_only.to_string(), &pct(summary.catalog_only)]);
    table.add_row(vec!["Codeowners Only", &summary.codeowners_only.to_string(), &pct(summary.codeowners_only)]);
    table.add_row(vec!["Stale", &summary.stale.to_string(), &pct(summary.stale)]);
    table.add_row(vec!["Missing", &summary.missing.to_string(), &pct(summary.missing)]);
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
        "CODEOWNERS Team",
        "Last Push",
        "Notes",
    ]);

    for repo in repos {
        let pushed = repo
            .pushed_at
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "-".to_string());

        table.add_row(vec![
            &repo.repo_name,
            &repo.alignment.to_string(),
            repo.catalog_owner.as_deref().unwrap_or("-"),
            repo.codeowners_team.as_deref().unwrap_or("-"),
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
    println!(
        "CODEOWNERS: {}{}",
        repo.codeowners_team.as_deref().unwrap_or("(none)"),
        match repo.codeowners_team_exists {
            Some(true) => " (team exists)",
            Some(false) => " (team NOT found)",
            None => "",
        }
    );
    if let Some(pushed) = repo.pushed_at {
        println!("Last Push:  {}", pushed.format("%Y-%m-%d %H:%M UTC"));
    }
    if !repo.notes.is_empty() {
        println!("Notes:      {}", repo.notes.join("; "));
    }
}
