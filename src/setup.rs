use std::path::{Path, PathBuf};

use crate::adapters::{get_adapter, resolve_target_dir, SetupLevel};
use crate::config::{default_setup, read_setup, write_setup, SetupFile};
use crate::error::SkmError;
use crate::store::profiles::load_profile;
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
    let profile = load_profile(store, profile_name)?;
    if !profile.skill.is_empty() {
        return Ok(());
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

pub fn set_setup_agent(setup: &mut SetupFile, agent: &str) -> Result<(), SkmError> {
    get_adapter(agent)?;
    setup.placement.agent = agent.to_string();
    Ok(())
}

pub fn write_project_setup(cwd: &Path, agent: &str) -> Result<PathBuf, SkmError> {
    let path = crate::config::project_setup_path(cwd);
    let setup = default_setup(agent);
    write_setup(&path, &setup)?;
    Ok(path)
}

pub fn target_dir_for_setup(setup: &SelectedSetup) -> Result<(String, PathBuf), SkmError> {
    resolve_target_dir(
        &setup.setup.placement.agent,
        setup.level,
        &setup.project_root,
    )
}
