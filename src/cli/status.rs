use std::env;
use std::io::{self, Write};

use crate::color::color_stdout;

use dialoguer::console::style;

use crate::adapters::format_agent_skills_path;
use crate::cli::output::{
    write_json, StatusAgentJson, StatusConflictJson, StatusJson, StatusSkillJson,
};
use crate::error::SkmError;
use crate::progress::display_path;
use crate::setup::select_command_setup;
use crate::store::StorePaths;
use crate::sync::{collect_status, AgentStatus, PlacementConflict, PlacementStatus};

pub fn run_status(store: &StorePaths, force_user: bool, json: bool) -> Result<(), SkmError> {
    let cwd = env::current_dir()?;
    let selected =
        select_command_setup(&cwd, force_user).map_err(|e| e.op("loading config file"))?;
    let reports = collect_status(store, &selected).map_err(|e| e.op("collecting linked skills"))?;

    let mut paths = Vec::with_capacity(reports.len());
    for report in &reports {
        paths.push(format_agent_skills_path(
            &report.agent,
            selected.level,
            &selected.project_root,
        )?);
    }

    if json {
        let payload = StatusJson {
            agents: reports
                .iter()
                .zip(&paths)
                .map(|(report, skills_path)| StatusAgentJson {
                    agent: &report.agent,
                    skills_path: skills_path.clone(),
                    skills: report
                        .linked
                        .iter()
                        .map(|placement| StatusSkillJson {
                            name: placement.name.clone(),
                            source: display_path(&placement.source),
                        })
                        .collect(),
                    conflicts: report
                        .conflicts
                        .iter()
                        .map(|conflict| StatusConflictJson {
                            name: conflict.name.clone(),
                            store_id: conflict.store_id.clone(),
                            reason: "conflicted",
                        })
                        .collect(),
                })
                .collect(),
            profiles: &selected.setup.profile.active,
        };
        write_json(&payload).map_err(|e| SkmError::Usage(format!("failed to encode JSON: {e}")))?;
        return Ok(());
    }

    let color = color_stdout();

    print_target_agents(&reports, &paths, color)?;
    print_active_profiles(&selected.setup.profile.active, color)?;
    writeln!(io::stdout())?;

    // One agent needs no per-agent headings, so its placements stay where they have always
    // been: directly under `Linked` / `Conflicts`.
    let grouped = reports.len() > 1;
    for (index, report) in reports.iter().enumerate() {
        if grouped {
            if index > 0 {
                writeln!(io::stdout())?;
            }
            print_agent_heading(&report.agent, color)?;
        }
        print_report(report, if grouped { "  " } else { "" }, color)?;
    }

    Ok(())
}

fn print_report(report: &AgentStatus, indent: &str, color: bool) -> io::Result<()> {
    if report.linked.is_empty() && report.conflicts.is_empty() {
        return print_empty(indent, color);
    }

    if !report.linked.is_empty() {
        print_section("Linked", indent, color)?;
        for placement in &report.linked {
            print_placement(placement, indent, color)?;
        }
    }

    if !report.conflicts.is_empty() {
        if !report.linked.is_empty() {
            writeln!(io::stdout())?;
        }
        print_section("Conflicts", indent, color)?;
        for conflict in &report.conflicts {
            print_conflict(conflict, indent, color)?;
        }
    }

    Ok(())
}

fn print_section(title: &str, indent: &str, color: bool) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if color {
        writeln!(out, "{indent}{}", style(title).dim())?;
    } else {
        writeln!(out, "{indent}{title}")?;
    }
    Ok(())
}

fn print_agent_heading(agent: &str, color: bool) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if color {
        writeln!(out, "{}", style(agent).cyan().bold())?;
    } else {
        writeln!(out, "{agent}")?;
    }
    Ok(())
}

fn print_target_agents(reports: &[AgentStatus], paths: &[String], color: bool) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if color {
        writeln!(out, "{}", style("Target agents:").dim())?;
    } else {
        writeln!(out, "Target agents:")?;
    }
    for (report, skills_path) in reports.iter().zip(paths) {
        if color {
            writeln!(
                out,
                "  {} {}",
                style(&report.agent).cyan().bold(),
                style(format!("({skills_path})")).dim()
            )?;
        } else {
            writeln!(out, "  {} ({skills_path})", report.agent)?;
        }
    }
    Ok(())
}

fn print_active_profiles(profiles: &[String], color: bool) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if color {
        write!(out, "{}", style("Active profiles:").dim())?;
        if profiles.is_empty() {
            writeln!(out, " {}", style("(none)").dim())?;
        } else {
            writeln!(
                out,
                " {}",
                style(profiles.join(", ")).cyan().bold()
            )?;
        }
    } else if profiles.is_empty() {
        writeln!(out, "Active profiles: (none)")?;
    } else {
        writeln!(out, "Active profiles: {}", profiles.join(", "))?;
    }
    Ok(())
}

fn print_empty(indent: &str, color: bool) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if color {
        writeln!(out, "{indent}{}", style("  (no skills linked)").dim())?;
    } else {
        writeln!(out, "{indent}  (no skills linked)")?;
    }
    Ok(())
}

fn print_placement(placement: &PlacementStatus, indent: &str, color: bool) -> io::Result<()> {
    let source = display_path(&placement.source);
    let mut out = io::stdout().lock();
    if color {
        writeln!(
            out,
            "{indent}  {} {} {}",
            style(&placement.name).green(),
            style("→").dim(),
            style(source).dim()
        )?;
    } else {
        writeln!(out, "{indent}  {} -> {source}", placement.name)?;
    }
    Ok(())
}

fn print_conflict(conflict: &PlacementConflict, indent: &str, color: bool) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if color {
        writeln!(
            out,
            "{indent}  {} {}",
            style(&conflict.name).yellow(),
            style("(conflicted — non-skm entry present)").dim()
        )?;
    } else {
        writeln!(
            out,
            "{indent}  {} (conflicted — non-skm entry present)",
            conflict.name
        )?;
    }
    Ok(())
}
