use std::env;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::Path;

use dialoguer::Confirm;

use crate::adapters::{get_adapter, known_agent_ids, resolve_target_dirs, AgentTarget};
use crate::error::SkmError;
use crate::progress;
use crate::progress::display_path;
use crate::setup::{select_project_setup_raw, SelectedSetup};
use crate::store::StorePaths;
use crate::sync::{clear_managed_exclude, unwire_all};

pub fn run_destroy(store: &StorePaths, force: bool, dry_run: bool) -> Result<(), SkmError> {
    let cwd = env::current_dir()?;
    let selected = select_project_setup_raw(&cwd).map_err(|e| e.op("loading config file"))?;
    warn_if_profile_missing(store, &selected);
    let targets = destroy_targets(&selected)?;

    if dry_run {
        preview_destroy(&selected, &targets, store)?;
        return Ok(());
    }

    if !force && !confirm_destroy(&selected.path)? {
        return Ok(());
    }

    let store_root = store.canonical_root();
    for target in &targets {
        if !target.dir.is_dir() {
            continue;
        }
        progress::step(format!(
            "removing {} links from {}",
            target.agent,
            display_path(&target.dir)
        ));
        unwire_all(&target.dir, &store_root, false)
            .map_err(|e| e.op("removing store-owned skill links"))?;
    }

    clear_managed_exclude(&selected.project_root, false)
        .map_err(|e| e.op("updating local git exclude"))?;

    fs::remove_file(&selected.path).map_err(|e| SkmError::from(e).op("removing config file"))?;
    progress::step(format!("removed {}", display_path(&selected.path)));
    Ok(())
}

fn destroy_targets(selected: &SelectedSetup) -> Result<Vec<AgentTarget>, SkmError> {
    for agent in selected.setup.placement.resolved_agents() {
        if get_adapter(&agent).is_err() {
            progress::warn(format!(
                "unknown agent `{agent}`; store-owned links in its directory were not cleaned"
            ));
        }
    }

    // Every adapter skm can place into, not only `placement.agents`. An empty or stale list
    // (hand-edited, or a dropped agent never passed through `remove-agent`) would otherwise
    // delete `.skm.toml` and leave store-owned links behind with no retry path.
    let agents: Vec<String> = known_agent_ids()
        .iter()
        .map(|agent| (*agent).to_string())
        .collect();
    resolve_target_dirs(&agents, selected.level, &selected.project_root)
}

fn warn_if_profile_missing(store: &StorePaths, selected: &SelectedSetup) {
    if selected.setup.profile.active.is_empty() {
        progress::warn("profile not found");
        return;
    }
    for name in &selected.setup.profile.active {
        if !store.profile_file(name).is_file() {
            progress::warn(format!("profile not found: {name}"));
        }
    }
}

fn preview_destroy(
    selected: &SelectedSetup,
    targets: &[AgentTarget],
    store: &StorePaths,
) -> Result<(), SkmError> {
    let store_root = store.canonical_root();
    for target in targets {
        if !target.dir.is_dir() {
            continue;
        }
        progress::step(format!(
            "(dry-run) {} {}",
            target.agent,
            display_path(&target.dir)
        ));
        unwire_all(&target.dir, &store_root, true)?;
    }
    clear_managed_exclude(&selected.project_root, true)?;
    progress::step(format!(
        "(dry-run) removing {}",
        display_path(&selected.path)
    ));
    Ok(())
}

fn confirm_destroy(setup_path: &Path) -> Result<bool, SkmError> {
    if !io::stdin().is_terminal() {
        return Err(SkmError::RefuseNonInteractiveDestroy);
    }

    Confirm::new()
        .with_prompt(format!(
            "Destroy the skm setup at {}?\n\
             This removes store-owned skill links and the managed git exclude block.\n\
             The skill store is not deleted.",
            display_path(setup_path)
        ))
        .default(false)
        .interact_opt()
        .map(|choice| choice.unwrap_or(false))
        .map_err(|_| SkmError::SelectionCancelled)
}
