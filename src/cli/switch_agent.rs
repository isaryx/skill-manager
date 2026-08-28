use std::env;
use std::io::{self, IsTerminal};
use std::path::Path;

use dialoguer::Confirm;

use crate::adapters::{get_adapter, interactive_switch_agent, SetupLevel};
use crate::cli::Agent;
use crate::config::write_setup;
use crate::error::SkmError;
use crate::progress;
use crate::setup::{select_setup, set_setup_agent};
use crate::store::StorePaths;
use crate::sync::{reconcile_with_setup, ReconcileOptions};

pub fn run_switch_agent(
    store: &StorePaths,
    agent: Option<Agent>,
    force_user: bool,
) -> Result<(), SkmError> {
    let cwd = env::current_dir()?;
    let mut selected = select_setup(&cwd, force_user).map_err(|e| e.op("loading config file"))?;

    let current = selected.setup.placement.agent.as_str();
    let new_agent = resolve_switch_agent(agent, current, selected.level, &selected.project_root)?;

    if selected.setup.placement.agent == new_agent {
        eprintln!("agent unchanged: {new_agent}");
        return Ok(());
    }

    let sync = should_sync_after_switch(selected.setup.profile.active.is_some())?;

    progress::step(format!("updating setup to {new_agent}"));
    set_setup_agent(&mut selected.setup, &new_agent)?;

    if sync {
        progress::step("syncing skills");
        reconcile_with_setup(store, &selected, None, ReconcileOptions::default())
            .map_err(|e| e.op("syncing skills"))?;
    }

    write_setup(&selected.path, &selected.setup).map_err(|e| e.op("writing config file"))?;
    println!("switched agent to {new_agent}");

    Ok(())
}

fn resolve_switch_agent(
    agent: Option<Agent>,
    current: &str,
    level: SetupLevel,
    project_root: &Path,
) -> Result<String, SkmError> {
    if let Some(agent) = agent {
        get_adapter(agent.as_str())?;
        return Ok(agent.as_str().to_string());
    }

    interactive_switch_agent(current, level, project_root)
}

fn should_sync_after_switch(has_active_profile: bool) -> Result<bool, SkmError> {
    if !has_active_profile {
        return Ok(false);
    }
    if !io::stdin().is_terminal() {
        return Ok(true);
    }

    Confirm::new()
        .with_prompt("Sync skills to the new agent now?")
        .default(true)
        .interact_opt()
        .map(|choice| choice.unwrap_or(false))
        .map_err(|_| SkmError::SelectionCancelled)
}
