use std::env;
use std::path::{Path, PathBuf};

use crate::adapters::{
    confirm_agent_skills_dirs_if_nonempty, interactive_select_agents,
    interactive_select_store_location, SetupLevel,
};
use crate::cli::{unique_agent_ids, Agent};
use crate::config::{
    default_setup, default_store_root, project_setup_path, read_app_config, read_setup,
    try_read_app_config, write_app_config, write_setup,
};
use crate::error::SkmError;
use crate::progress::{self, display_path};
use crate::setup::set_setup_agents;
use crate::store::{inspect_store_path, prepare_store, StoreState};

pub fn run_init(
    cli_store: Option<&Path>,
    agents: &[Agent],
    force: bool,
    accept_existing_skills: bool,
) -> Result<(), SkmError> {
    let cwd = env::current_dir()?;
    let setup_path = project_setup_path(&cwd);
    if setup_path.is_file() && !force {
        return Err(SkmError::SetupExists(setup_path));
    }

    let agents = resolve_init_agents(agents, &cwd)?;
    let store_path = resolve_store_location(cli_store)?;

    match inspect_store_path(&store_path).map_err(|e| e.op("inspecting skill store"))? {
        StoreState::Absent => {
            progress::step(format!(
                "creating skill store at {}",
                display_path(&store_path)
            ));
        }
        StoreState::Valid => {
            progress::step(format!(
                "using skill store at {}",
                display_path(&store_path)
            ));
        }
        StoreState::Invalid(_) => {
            progress::step(format!(
                "validating skill store at {}",
                display_path(&store_path)
            ));
        }
    }

    let store = prepare_store(&store_path).map_err(|e| e.op("preparing skill store"))?;

    if explicit_app_config_write(cli_store.is_some()) {
        progress::step("saving store config");
        write_app_config(store.root())?;
    }

    progress::step(format!(
        "validating skills directories for {}",
        agents.join(", ")
    ));
    confirm_agent_skills_dirs_if_nonempty(&agents, &cwd, accept_existing_skills)?;

    progress::step("writing .skm.toml");
    let setup = if setup_path.is_file() && force {
        let mut existing = read_setup(&setup_path).map_err(|e| e.op("reading config file"))?;
        set_setup_agents(&mut existing, &agents)?;
        existing
    } else {
        default_setup(&agents)
    };
    write_setup(&setup_path, &setup).map_err(|e| e.op("writing config file"))?;

    Ok(())
}

fn resolve_store_location(cli_store: Option<&Path>) -> Result<PathBuf, SkmError> {
    if let Some(path) = cli_store {
        return Ok(path.to_path_buf());
    }

    if let Ok(env) = std::env::var("SKM_STORE") {
        if !env.is_empty() {
            return Ok(PathBuf::from(env));
        }
    }

    if let Some(config) = try_read_app_config() {
        return Ok(config.store.path);
    }

    interactive_select_store_location(&default_store_root())
}

fn explicit_app_config_write(explicit: bool) -> bool {
    explicit || read_app_config().is_err()
}

/// The agents to place into: whatever `--agent` named, or an interactive pick.
///
/// `--agent` repeats, so duplicates are dropped here rather than written to the config.
fn resolve_init_agents(agents: &[Agent], cwd: &Path) -> Result<Vec<String>, SkmError> {
    if agents.is_empty() {
        return interactive_select_agents(&[], SetupLevel::Project, cwd);
    }

    unique_agent_ids(agents)
}
