use std::path::{Path, PathBuf};

use crate::adapters::{
    canonical_agent_id, get_adapter, resolve_target_dirs, AgentTarget, SetupLevel,
};
use crate::config::{default_setup, read_setup, write_setup, SetupFile};
use crate::error::SkmError;
use crate::store::extends::flatten_skill_ids;
use crate::store::StorePaths;

#[derive(Debug, Clone)]
pub struct SelectedSetup {
    pub path: PathBuf,
    pub setup: SetupFile,
    pub level: SetupLevel,
    pub project_root: PathBuf,
}

pub fn select_setup_lenient(cwd: &Path, force_user: bool) -> Result<SelectedSetup, SkmError> {
    let user_path = crate::config::user_setup_path();
    let project_path = crate::config::project_setup_path(cwd);

    if force_user {
        let setup = crate::config::read_setup_raw(&user_path)?;
        return Ok(SelectedSetup {
            path: user_path,
            setup,
            level: SetupLevel::User,
            project_root: cwd.to_path_buf(),
        });
    }

    if project_path.is_file() {
        let setup = crate::config::read_setup_raw(&project_path)?;
        return Ok(SelectedSetup {
            path: project_path,
            setup,
            level: SetupLevel::Project,
            project_root: cwd.to_path_buf(),
        });
    }

    let setup = crate::config::read_setup_raw(&user_path)?;
    Ok(SelectedSetup {
        path: user_path,
        setup,
        level: SetupLevel::User,
        project_root: cwd.to_path_buf(),
    })
}

pub fn select_setup(cwd: &Path, force_user: bool) -> Result<SelectedSetup, SkmError> {
    let user_path = crate::config::user_setup_path();
    let project_path = crate::config::project_setup_path(cwd);

    if force_user {
        let setup = read_setup(&user_path)?;
        return Ok(SelectedSetup {
            path: user_path,
            setup,
            level: SetupLevel::User,
            project_root: cwd.to_path_buf(),
        });
    }

    if project_path.is_file() {
        let setup = read_setup(&project_path)?;
        return Ok(SelectedSetup {
            path: project_path,
            setup,
            level: SetupLevel::Project,
            project_root: cwd.to_path_buf(),
        });
    }

    let setup = read_setup(&user_path)?;
    Ok(SelectedSetup {
        path: user_path,
        setup,
        level: SetupLevel::User,
        project_root: cwd.to_path_buf(),
    })
}

pub fn select_project_setup(cwd: &Path) -> Result<SelectedSetup, SkmError> {
    let project_path = crate::config::project_setup_path(cwd);
    if !project_path.is_file() {
        return Err(SkmError::SetupNotFound(project_path));
    }

    let setup = read_setup(&project_path)?;
    Ok(SelectedSetup {
        path: project_path,
        setup,
        level: SetupLevel::Project,
        project_root: cwd.to_path_buf(),
    })
}

/// Load `./.skm.toml` without validating agents, so a broken setup can still be torn down.
pub fn select_project_setup_raw(cwd: &Path) -> Result<SelectedSetup, SkmError> {
    let project_path = crate::config::project_setup_path(cwd);
    if !project_path.is_file() {
        return Err(SkmError::SetupNotFound(project_path));
    }

    let setup = crate::config::read_setup_raw(&project_path)?;
    Ok(SelectedSetup {
        path: project_path,
        setup,
        level: SetupLevel::Project,
        project_root: cwd.to_path_buf(),
    })
}

pub fn set_active_profile(setup: &mut SelectedSetup, profile_name: &str) -> Result<(), SkmError> {
    setup.setup.profile.active = Some(profile_name.to_string());
    write_setup(&setup.path, &setup.setup)?;
    Ok(())
}

pub fn clear_active_profile_if_empty(
    cwd: &Path,
    store: &StorePaths,
    profile_name: &str,
) -> Result<(), SkmError> {
    // Emptiness is a property of the resolved set, so a profile whose skills all come from
    // `extends` stays active. A broken extend link leaves us unable to tell, so do nothing
    // rather than deactivating a profile that may well be fine.
    match flatten_skill_ids(store, profile_name) {
        Ok(skills) if skills.is_empty() => {}
        _ => return Ok(()),
    }

    use crate::config::{project_setup_path, read_setup, user_setup_path};

    for path in [project_setup_path(cwd), user_setup_path()] {
        if !path.is_file() {
            continue;
        }
        let mut setup = read_setup(&path)?;
        if setup.profile.active.as_deref() == Some(profile_name) {
            setup.profile.active = None;
            write_setup(&path, &setup)?;
        }
    }

    Ok(())
}

/// Replace the setup's target agents, canonicalizing aliases and dropping repeats so the
/// written file says exactly what will be placed into.
pub fn set_setup_agents(setup: &mut SetupFile, agents: &[String]) -> Result<(), SkmError> {
    if agents.is_empty() {
        return Err(SkmError::NoTargetAgents);
    }

    let mut canonical: Vec<String> = Vec::with_capacity(agents.len());
    for agent in agents {
        get_adapter(agent)?;
        let agent = canonical_agent_id(agent).to_string();
        if !canonical.contains(&agent) {
            canonical.push(agent);
        }
    }

    setup.placement.agents = canonical;
    Ok(())
}

pub fn write_project_setup(cwd: &Path, agents: &[String]) -> Result<PathBuf, SkmError> {
    let path = crate::config::project_setup_path(cwd);
    let setup = default_setup(agents);
    write_setup(&path, &setup)?;
    Ok(path)
}

/// Every agent directory this setup places into, in config order.
pub fn target_dirs_for_setup(setup: &SelectedSetup) -> Result<Vec<AgentTarget>, SkmError> {
    resolve_target_dirs(
        &setup.setup.placement.resolved_agents(),
        setup.level,
        &setup.project_root,
    )
}
