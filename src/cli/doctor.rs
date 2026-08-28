use dialoguer::console::style;

use crate::cli::output::write_json;
use crate::color::color_stdout;
use crate::doctor::{run_checks, Issue, Report, Severity};
use crate::error::SkmError;
use crate::store::StorePaths;

pub fn run_doctor(store: &StorePaths, force_user: bool, json: bool) -> Result<i32, SkmError> {
    let report = run_checks(store, force_user)?;

    if json {
        write_json(&report).map_err(|e| SkmError::Usage(format!("failed to encode JSON: {e}")))?;
    } else {
        print_human(&report);
    }

    Ok(report.exit_code())
}

fn print_human(report: &Report) {
    if report.issues.is_empty() {
        println!("No issues found.");
        return;
    }

    let count = report.issues.len();
    let noun = if count == 1 { "issue" } else { "issues" };
    println!("{count} {noun} found:\n");

    let mut sorted: Vec<&Issue> = report.issues.iter().collect();
    sorted.sort_by(|a, b| {
        severity_rank(a.severity)
            .cmp(&severity_rank(b.severity))
            .then_with(|| a.code.cmp(b.code))
    });

    let color = color_stdout();
    for issue in sorted {
        let label = if color {
            match issue.severity {
                Severity::Error => style("ERROR").red().bold().to_string(),
                Severity::Warn => style("WARN").yellow().bold().to_string(),
                Severity::Info => style("INFO").dim().to_string(),
            }
        } else {
            match issue.severity {
                Severity::Error => "ERROR".to_string(),
                Severity::Warn => "WARN".to_string(),
                Severity::Info => "INFO".to_string(),
            }
        };
        println!("  {label}  {}", issue.code);
        println!("         {}", issue.message);
        println!();
    }
}

fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 0,
        Severity::Warn => 1,
        Severity::Info => 2,
    }
}
