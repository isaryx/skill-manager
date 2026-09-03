use std::env;
use std::io::{self, IsTerminal};

use dialoguer::Confirm;

use crate::adapters::{
    canonical_agent_id, get_adapter, interactive_select_agents, resolve_target_dirs,
    AgentTarget,
};
use crate::cli::Agent;
use crate::config::write_setup;
use crate::error::SkmError;
use crate::progress;
use crate::progress::display_path;
use crate::setup::{select_command_setup, set_setup_agents, target_dirs_for_setup};
use crate::store::StorePaths;
use crate::sync::{reconcile_with_setup, refresh_local_exclude, unwire_all, ReconcileOptions};
use crate::util::paths_equal;

pub fn run_use_agents(store: &StorePaths, force_user: bool) -> Result<(), SkmError> {
    if !io::stdin().is_terminal() {
        return Err(SkmError::NotATty);
    }

    let cwd = env::current_dir()?;
    let selected =
        select_command_setup(&cwd, force_user).map_err(|e| e.op("loading config file"))?;
    let chosen = interactive_select_agents(
        &selected.setup.placement.agents,
        selected.level,
        &selected.project_root,
    )?;
    apply_target_agents(store, selected, &chosen)
}

pub fn run_add_agent(
    store: &StorePaths,
    agent: &Agent,
    force_user: bool,
) -> Result<(), SkmError> {
    let cwd = env::current_dir()?;
    let selected =
        select_command_setup(&cwd, force_user).map_err(|e| e.op("loading config file"))?;
    let chosen = build_chosen_for_add(&selected.setup.placement.agents, agent)?;
    apply_target_agents(store, selected, &chosen)
}

pub fn run_remove_agent(
    store: &StorePaths,
    agent: &Agent,
    force_user: bool,
) -> Result<(), SkmError> {
    let cwd = env::current_dir()?;
    let selected =
        select_command_setup(&cwd, force_user).map_err(|e| e.op("loading config file"))?;
    let chosen = build_chosen_for_remove(&selected.setup.placement.agents, agent)?;
    apply_target_agents(store, selected, &chosen)
}

fn apply_target_agents(
    store: &StorePaths,
    mut selected: crate::setup::SelectedSetup,
    chosen: &[String],
) -> Result<(), SkmError> {
    // Compared against the ids as written, not the canonical ones, so replacing a legacy alias
    // (`codex`) with the id it maps to (`generic`) still rewrites the file. The order agents are
    // listed in does not change where anything is placed, so a reordered selection is the same
    // setup and not worth a rewrite.
    if same_agents(&selected.setup.placement.agents, chosen) {
        eprintln!(
            "target agents unchanged: {}",
            selected.setup.placement.agents.join(", ")
        );
        return Ok(());
    }

    let old_targets = target_dirs_for_setup(&selected).unwrap_or_default();
    let new_targets = resolve_target_dirs(chosen, selected.level, &selected.project_root)?;

    let dropped: Vec<AgentTarget> = old_targets
        .iter()
        .filter(|old| {
            !new_targets
                .iter()
                .any(|new| paths_equal(&old.dir, &new.dir))
        })
        .cloned()
        .collect();
    let gained = new_targets.iter().any(|new| {
        !old_targets
            .iter()
            .any(|old| paths_equal(&old.dir, &new.dir))
    });

    // Only a directory that is new to this setup has links missing from it. Dropping an agent
    // needs no sync — the directories that remain are already wired.
    let sync = gained && should_sync_after_change(!selected.setup.profile.active.is_empty())?;

    progress::step(format!("updating setup to {}", chosen.join(", ")));
    set_setup_agents(&mut selected.setup, chosen)?;

    if sync {
        progress::step("syncing skills");
        reconcile_with_setup(store, &selected, None, ReconcileOptions::default())
            .map_err(|e| e.op("syncing skills"))?;
    }

    // Before unwire: a failed write after cleanup used to leave the old agent list on disk,
    // and the next sync put the dropped links back. After a failed sync we have not written,
    // so the file still matches the directories we did not touch.
    write_setup(&selected.path, &selected.setup).map_err(|e| e.op("writing config file"))?;

    for target in &dropped {
        progress::step(format!(
            "removing {} links from {}",
            target.agent,
            display_path(&target.dir)
        ));
        unwire_all(&target.dir, &store.canonical_root(), false)
            .map_err(|e| e.op("cleaning up a dropped agent's skills"))?;
    }

    // A sync already rewrote the exclude block for the new agent set. Without one, the block
    // still lists the dropped agents' paths, so rebuild it from the links left on disk.
    if !sync && !dropped.is_empty() {
        refresh_local_exclude(&selected, &store.canonical_root())
            .map_err(|e| e.op("updating local git exclude"))?;
    }

    println!("target agents: {}", chosen.join(", "));

    Ok(())
}

fn build_chosen_for_add(current: &[String], agent: &Agent) -> Result<Vec<String>, SkmError> {
    let id = agent.as_str();
    get_adapter(id)?;
    let canonical = canonical_agent_id(id).to_string();
    if current
        .iter()
        .any(|agent| canonical_agent_id(agent) == canonical)
    {
        return Ok(current.to_vec());
    }
    let mut agents = current.to_vec();
    agents.push(id.to_string());
    Ok(agents)
}

fn build_chosen_for_remove(current: &[String], agent: &Agent) -> Result<Vec<String>, SkmError> {
    let id = agent.as_str();
    get_adapter(id)?;
    let canonical = canonical_agent_id(id).to_string();
    if !current
        .iter()
        .any(|agent| canonical_agent_id(agent) == canonical)
    {
        return Err(SkmError::AgentNotTarget(id.to_string()));
    }
    let remaining: Vec<String> = current
        .iter()
        .filter(|agent| canonical_agent_id(agent) != canonical)
        .cloned()
        .collect();
    if remaining.is_empty() {
        return Err(SkmError::LastTargetAgentRemoval);
    }
    Ok(remaining)
}

/// Same agents, regardless of the order they were picked in.
fn same_agents(current: &[String], chosen: &[String]) -> bool {
    current.len() == chosen.len() && chosen.iter().all(|agent| current.contains(agent))
}

fn should_sync_after_change(has_active_profile: bool) -> Result<bool, SkmError> {
    if !has_active_profile {
        return Ok(false);
    }
    if !io::stdin().is_terminal() {
        return Ok(true);
    }

    Confirm::new()
        .with_prompt("Sync skills to the new agents now?")
        .default(true)
        .interact_opt()
        .map(|choice| choice.unwrap_or(false))
        .map_err(|_| SkmError::SelectionCancelled)
}
