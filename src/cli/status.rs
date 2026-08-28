use std::env;
use std::io::{self, Write};

use crate::color::color_stdout;

use dialoguer::console::style;

use crate::adapters::format_agent_skills_path;
use crate::cli::output::{write_json, StatusConflictJson, StatusJson, StatusSkillJson};
use crate::error::SkmError;
use crate::progress::display_path;
use crate::setup::{select_project_setup, select_setup};
use crate::store::StorePaths;
use crate::sync::{collect_status, PlacementConflict, PlacementStatus};

pub fn run_status(store: &StorePaths, force_user: bool, json: bool) -> Result<(), SkmError> {
    let cwd = env::current_dir()?;
    let selected = if force_user {
        select_setup(&cwd, true).map_err(|e| e.op("loading config file"))?
    } else {
        select_project_setup(&cwd).map_err(|e| e.op("loading config file"))?
    };
    let report = collect_status(store, &selected).map_err(|e| e.op("collecting linked skills"))?;

    let agent = &selected.setup.placement.agent;
    let skills_path = format_agent_skills_path(agent, selected.level, &selected.project_root)?;

    if json {
        let payload = StatusJson {
            agent,
            skills_path,
            profile: selected.setup.profile.active.as_deref(),
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
        };
        write_json(&payload).map_err(|e| SkmError::Usage(format!("failed to encode JSON: {e}")))?;
        return Ok(());
    }

    let color = color_stdout();

    print_target_agent(agent, &skills_path, color)?;
    print_active_profile(selected.setup.profile.active.as_deref(), color)?;
    writeln!(io::stdout())?;

    if report.linked.is_empty() && report.conflicts.is_empty() {
        print_empty(color)?;
        return Ok(());
    }

    if !report.linked.is_empty() {
        print_section("Linked", color)?;
        for placement in &report.linked {
            print_placement(placement, color)?;
        }
    }

    if !report.conflicts.is_empty() {
        if !report.linked.is_empty() {
            writeln!(io::stdout())?;
        }
        print_section("Conflicts", color)?;
        for conflict in &report.conflicts {
            print_conflict(conflict, color)?;
        }
    }

    Ok(())
}

fn print_section(title: &str, color: bool) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if color {
        writeln!(out, "{}", style(title).dim())?;
    } else {
        writeln!(out, "{title}")?;
    }
    Ok(())
}

fn print_target_agent(agent: &str, skills_path: &str, color: bool) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if color {
        write!(out, "{}", style("Target agent:").dim())?;
        write!(out, " {}", style(agent).cyan().bold())?;
        writeln!(out, " {}", style(format!("({skills_path})")).dim())?;
    } else {
        writeln!(out, "Target agent: {agent} ({skills_path})")?;
    }
    Ok(())
}

fn print_active_profile(profile: Option<&str>, color: bool) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if color {
        write!(out, "{}", style("Active profile:").dim())?;
        match profile {
            Some(name) => writeln!(out, " {}", style(name).cyan().bold())?,
            None => writeln!(out, " {}", style("(none)").dim())?,
        }
    } else {
        match profile {
            Some(name) => writeln!(out, "Active profile: {name}")?,
            None => writeln!(out, "Active profile: (none)")?,
        }
    }
    Ok(())
}

fn print_empty(color: bool) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if color {
        writeln!(out, "{}", style("  (no skills linked)").dim())?;
    } else {
        writeln!(out, "  (no skills linked)")?;
    }
    Ok(())
}

fn print_placement(placement: &PlacementStatus, color: bool) -> io::Result<()> {
    let source = display_path(&placement.source);
    let mut out = io::stdout().lock();
    if color {
        writeln!(
            out,
            "  {} {} {}",
            style(&placement.name).green(),
            style("→").dim(),
            style(source).dim()
        )?;
    } else {
        writeln!(out, "  {} -> {source}", placement.name)?;
    }
    Ok(())
}

fn print_conflict(conflict: &PlacementConflict, color: bool) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if color {
        writeln!(
            out,
            "  {} {}",
            style(&conflict.name).yellow(),
            style("(conflicted — non-skm entry present)").dim()
        )?;
    } else {
        writeln!(
            out,
            "  {} (conflicted — non-skm entry present)",
            conflict.name
        )?;
    }
    Ok(())
}
