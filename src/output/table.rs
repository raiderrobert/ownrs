use comfy_table::{ContentArrangement, Table};
use unicode_width::UnicodeWidthStr;

use crate::reconcile::types::{AlignmentStatus, AuditSummary, RepoOwnership};

const MAX_COL_WIDTH: usize = 30;

/// Options controlling table rendering.
pub struct TableOptions {
    pub wide: bool,
    pub sort_columns: Vec<String>,
    pub team_filter: Option<String>,
}

#[derive(Clone, Copy)]
enum Column {
    Repo,
    Status,
    CatalogOwner,
    CodeownersTeams,
    LastPush,
    AdminTeams,
    Notes,
}

impl Column {
    fn header(&self) -> &'static str {
        match self {
            Column::Repo => "REPO",
            Column::Status => "STATUS",
            Column::CatalogOwner => "CATALOG OWNER",
            Column::CodeownersTeams => "CODEOWNERS TEAMS",
            Column::LastPush => "LAST PUSH",
            Column::AdminTeams => "ADMIN TEAMS",
            Column::Notes => "NOTES",
        }
    }

    fn sort_key(&self) -> &'static str {
        match self {
            Column::Repo => "repo",
            Column::Status => "status",
            Column::CatalogOwner => "catalog-owner",
            Column::CodeownersTeams => "codeowners-teams",
            Column::LastPush => "last-push",
            Column::AdminTeams => "admin-teams",
            Column::Notes => "notes",
        }
    }
}

fn default_columns() -> Vec<Column> {
    vec![
        Column::Repo,
        Column::Status,
        Column::CatalogOwner,
        Column::CodeownersTeams,
        Column::LastPush,
    ]
}

fn wide_columns() -> Vec<Column> {
    vec![
        Column::Repo,
        Column::Status,
        Column::CatalogOwner,
        Column::CodeownersTeams,
        Column::LastPush,
        Column::AdminTeams,
        Column::Notes,
    ]
}

/// Render the main table output as a String.
pub fn render_table(repos: &[RepoOwnership], opts: &TableOptions) -> String {
    let mut sorted: Vec<RepoOwnership> = repos.to_vec();
    sort_repos(&mut sorted, &opts.sort_columns);

    let columns = if opts.wide {
        wide_columns()
    } else {
        default_columns()
    };

    // Title line
    let filter_label = opts.team_filter.as_deref().unwrap_or("all");
    let title = format!("repos({})[{}]", filter_label, sorted.len());

    // Build all row data
    let rows: Vec<Vec<String>> = sorted.iter().map(|r| row_values(r, &columns)).collect();

    // Compute column widths (min of max content width and MAX_COL_WIDTH)
    let mut col_widths: Vec<usize> = columns
        .iter()
        .map(|c| UnicodeWidthStr::width(c.header()))
        .collect();
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            let w = UnicodeWidthStr::width(cell.as_str());
            if w > col_widths[i] {
                col_widths[i] = w;
            }
        }
    }
    // Cap at MAX_COL_WIDTH
    for w in &mut col_widths {
        if *w > MAX_COL_WIDTH {
            *w = MAX_COL_WIDTH;
        }
    }

    // Build header with sort arrow
    let primary_sort = opts.sort_columns.first().map(|s| s.to_lowercase());
    let headers: Vec<String> = columns
        .iter()
        .map(|col| {
            let h = col.header().to_string();
            if let Some(ref ps) = primary_sort {
                if col.sort_key() == ps.as_str() {
                    return format!("{}\u{2191}", h); // ↑
                }
            }
            h
        })
        .collect();

    // Recalculate widths including sort arrow
    for (i, header) in headers.iter().enumerate() {
        let w = UnicodeWidthStr::width(header.as_str());
        if w > col_widths[i] {
            col_widths[i] = w;
        }
        if col_widths[i] > MAX_COL_WIDTH {
            col_widths[i] = MAX_COL_WIDTH;
        }
    }

    let mut out = String::new();
    out.push_str(&title);
    out.push('\n');

    // Header line
    let header_line: Vec<String> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| pad_right(h, col_widths[i]))
        .collect();
    out.push_str(&header_line.join("  "));
    out.push('\n');

    // Data rows
    for row in &rows {
        let cells: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let t = truncate(cell, col_widths[i]);
                pad_right(&t, col_widths[i])
            })
            .collect();
        out.push_str(&cells.join("  "));
        out.push('\n');
    }

    // Tally footer
    let footer = tally_footer(&sorted);
    if !footer.is_empty() {
        out.push_str(&footer);
        out.push('\n');
    }

    out
}

/// Render the legacy summary table using comfy-table.
pub fn render_summary(summary: &AuditSummary) -> String {
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

    table.to_string()
}

/// Render repo names only, one per line, sorted alphabetically.
pub fn render_names(repos: &[RepoOwnership]) -> String {
    let mut names: Vec<&str> = repos.iter().map(|r| r.repo_name.as_str()).collect();
    names.sort();
    let mut out = String::new();
    for name in names {
        out.push_str(name);
        out.push('\n');
    }
    out
}

/// Print a single repo's details (kept unchanged from original).
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
            println!("\nUnresolved:        {}", suggestion.unresolved.join(", "));
        }
    }
}

// --- Internal helpers ---

fn sort_repos(repos: &mut [RepoOwnership], sort_columns: &[String]) {
    if sort_columns.is_empty() {
        // Default sort by repo name
        repos.sort_by(|a, b| a.repo_name.to_lowercase().cmp(&b.repo_name.to_lowercase()));
        return;
    }

    repos.sort_by(|a, b| {
        for col in sort_columns {
            let ord = match col.to_lowercase().as_str() {
                "repo" => a.repo_name.to_lowercase().cmp(&b.repo_name.to_lowercase()),
                "status" => a.alignment.to_string().cmp(&b.alignment.to_string()),
                "catalog-owner" => {
                    let a_val = a.catalog_owner.as_deref().unwrap_or("");
                    let b_val = b.catalog_owner.as_deref().unwrap_or("");
                    a_val.to_lowercase().cmp(&b_val.to_lowercase())
                }
                "codeowners-teams" => {
                    let a_val = a.codeowners_teams.join(", ");
                    let b_val = b.codeowners_teams.join(", ");
                    a_val.to_lowercase().cmp(&b_val.to_lowercase())
                }
                "last-push" => a.pushed_at.cmp(&b.pushed_at),
                "admin-teams" => {
                    let a_val = a.admin_teams.join(", ");
                    let b_val = b.admin_teams.join(", ");
                    a_val.to_lowercase().cmp(&b_val.to_lowercase())
                }
                "notes" => {
                    let a_val = a.notes.join("; ");
                    let b_val = b.notes.join("; ");
                    a_val.cmp(&b_val)
                }
                _ => std::cmp::Ordering::Equal,
            };
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    });
}

fn truncate(s: &str, max: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    // Need to truncate: take chars until we'd exceed max-1 width, then add …
    let mut result = String::new();
    let mut current_width = 0;
    let target = max - 1; // reserve 1 for …
    for ch in s.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + ch_width > target {
            break;
        }
        result.push(ch);
        current_width += ch_width;
    }
    result.push('\u{2026}'); // …
    result
}

fn pad_right(s: &str, width: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w >= width {
        return s.to_string();
    }
    let padding = width - w;
    format!("{}{}", s, " ".repeat(padding))
}

fn tally_footer(repos: &[RepoOwnership]) -> String {
    let total = repos.len();
    if total == 0 {
        return String::new();
    }

    let counts = [
        ("aligned", AlignmentStatus::Aligned),
        ("mismatched", AlignmentStatus::Mismatched),
        ("catalog-only", AlignmentStatus::CatalogOnly),
        ("codeowners-only", AlignmentStatus::CodeownersOnly),
        ("admin-only", AlignmentStatus::AdminOnly),
        ("stale", AlignmentStatus::Stale),
        ("missing", AlignmentStatus::Missing),
    ];

    let mut parts: Vec<String> = Vec::new();
    for (label, status) in &counts {
        let count = repos.iter().filter(|r| r.alignment == *status).count();
        if count > 0 {
            let pct = (count as f64 / total as f64 * 100.0).round() as usize;
            parts.push(format!("{} {} ({}%)", count, label, pct));
        }
    }

    parts.join(" \u{00b7} ") // middle dot ·
}

fn row_values(repo: &RepoOwnership, columns: &[Column]) -> Vec<String> {
    columns
        .iter()
        .map(|col| match col {
            Column::Repo => repo.repo_name.clone(),
            Column::Status => repo.alignment.to_string(),
            Column::CatalogOwner => repo.catalog_owner.as_deref().unwrap_or("-").to_string(),
            Column::CodeownersTeams => {
                if repo.codeowners_teams.is_empty() {
                    "-".to_string()
                } else {
                    repo.codeowners_teams.join(", ")
                }
            }
            Column::LastPush => repo
                .pushed_at
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "-".to_string()),
            Column::AdminTeams => {
                if repo.admin_teams.is_empty() {
                    "-".to_string()
                } else {
                    repo.admin_teams.join(", ")
                }
            }
            Column::Notes => {
                if repo.notes.is_empty() {
                    String::new()
                } else {
                    repo.notes.join("; ")
                }
            }
        })
        .collect()
}
