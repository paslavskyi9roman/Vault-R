//! Terminal rendering for `vault scan` and `vault health`.
//!
//! These build strings rather than printing directly so the one invariant that
//! matters can be asserted in a test: **no secret value ever reaches stdout**.
//! The core reports are already value-free by construction
//! (`vault_core::gitguard`, `vault_core::health`), and this layer must not
//! reintroduce one by, say, echoing a matched line.

use vault_core::gitguard::LeakReport;
use vault_core::health::HealthReport;

/// One repository's leak-guard result. Safe to paste into an issue or a chat,
/// which is most of the point of the feature.
pub fn leak_report_text(report: &LeakReport) -> String {
    let mut out = format!("{}\n", report.path);
    if let Some(note) = &report.note {
        out.push_str(&format!("  {note}\n"));
    }
    if report.findings.is_empty() {
        if report.git_root.is_some() {
            out.push_str(&format!(
                "  Clean — {} tracked file(s) searched.\n",
                report.files_scanned
            ));
        }
        return out;
    }
    for f in &report.findings {
        let marker = if f.severity == "critical" { "!!" } else { " !" };
        match f.line {
            Some(line) => out.push_str(&format!("  {marker} {}:{line}\n", f.path)),
            None => out.push_str(&format!("  {marker} {}\n", f.path)),
        }
        if let Some(key) = &f.key {
            match (&f.repo_name, &f.env_name) {
                (Some(repo), Some(env)) => {
                    out.push_str(&format!("     {key}  ({repo}/{env})\n"))
                }
                _ => out.push_str(&format!("     {key}\n")),
            }
        }
        out.push_str(&format!("     {}\n", f.detail));
    }
    let critical = report.findings.iter().filter(|f| f.severity == "critical").count();
    out.push_str(&format!(
        "  {} finding(s), {critical} critical, from {} tracked file(s) searched.\n",
        report.findings.len(),
        report.files_scanned
    ));
    out
}

pub fn health_report_text(label: &str, report: &HealthReport) -> String {
    let mut out = format!(
        "{label}: {} secret(s), {} empty, {} placeholder, {} stale, {} due for rotation.\n",
        report.total_secrets,
        report.empty_count,
        report.placeholder_count,
        report.stale_count,
        report.rotation_due_count
    );
    for row in &report.rows {
        out.push_str(&format!("  {}/{}  {}\n", row.repo_name, row.env_name, row.key));
        for issue in &row.issues {
            out.push_str(&format!("     - {}\n", issue.detail));
        }
    }
    if !report.duplicates.is_empty() {
        out.push_str("\nIdentical values that are not linked:\n");
        for group in &report.duplicates {
            let name = if group.key.is_empty() { "(different keys)" } else { &group.key };
            let places: Vec<String> = group
                .locations
                .iter()
                .map(|l| format!("{}/{}:{}", l.repo_name, l.env_name, l.key))
                .collect();
            out.push_str(&format!("  {name} — {}\n", places.join(", ")));
        }
        out.push_str("\nLink them so one edit updates them all: `link` on the row in the app.\n");
    }
    if report.rows.is_empty() && report.duplicates.is_empty() {
        out.push_str("Nothing to report.\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use vault_core::gitguard::LeakFinding;

    fn finding_naming(secret_key: &str) -> LeakFinding {
        LeakFinding {
            kind: "trackedValue".into(),
            severity: "critical".into(),
            path: "docker-compose.yml".into(),
            line: Some(4),
            key: Some(secret_key.into()),
            repo_name: Some("api-gateway".into()),
            env_name: Some("production".into()),
            detail: format!(
                "The value of {secret_key} from api-gateway/production appears in \
                 docker-compose.yml line 4, which git tracks."
            ),
            fix_pattern: None,
            needs_rotation: true,
        }
    }

    #[test]
    fn a_leak_report_names_the_key_and_location_but_never_the_value() {
        let report = LeakReport {
            path: "C:/code/api".into(),
            git_root: Some("C:/code/api".into()),
            note: None,
            files_scanned: 12,
            findings: vec![finding_naming("STRIPE_SECRET_KEY")],
        };
        let text = leak_report_text(&report);

        assert!(text.contains("STRIPE_SECRET_KEY"));
        assert!(text.contains("docker-compose.yml:4"));
        assert!(text.contains("api-gateway/production"));
        assert!(text.contains("1 finding(s), 1 critical"));
    }

    #[test]
    fn a_clean_report_says_so_with_the_file_count() {
        let report = LeakReport {
            path: "C:/code/api".into(),
            git_root: Some("C:/code/api".into()),
            note: None,
            files_scanned: 12,
            findings: Vec::new(),
        };
        assert!(leak_report_text(&report).contains("Clean — 12 tracked file(s)"));
    }

    #[test]
    fn a_directory_outside_git_renders_its_note_and_no_clean_claim() {
        let report = LeakReport {
            path: "C:/notes".into(),
            git_root: None,
            note: Some("Not a git repository — nothing here can be committed by accident.".into()),
            files_scanned: 0,
            findings: Vec::new(),
        };
        let text = leak_report_text(&report);
        assert!(text.contains("Not a git repository"));
        // claiming "clean" for a directory we never actually searched would be
        // a reassurance we have not earned
        assert!(!text.contains("Clean"));
    }
}
