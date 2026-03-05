use crate::reconcile::types::AuditSummary;

pub fn print_json(summary: &AuditSummary) {
    let json = serde_json::to_string_pretty(summary).unwrap_or_else(|e| {
        eprintln!("Failed to serialize JSON: {e}");
        "{}".to_string()
    });
    println!("{json}");
}
