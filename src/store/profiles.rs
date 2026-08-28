use std::fs;
use std::io::{self, IsTerminal};
use std::path::Path;

use dialoguer::MultiSelect;

use crate::config::{ProfileFile, ProfileSkillEntry};
use crate::error::SkmError;
use crate::store::skills::read_disabled_ids;
use crate::store::{list_pool_ids, StorePaths};
use crate::util::{validate_profile_name, validate_store_skill_id};

pub fn create_profile(
    store: &StorePaths,
    name: &str,
    skill_ids: &[String],
) -> Result<(), SkmError> {
    store.ensure_initialized()?;
    validate_profile_name(name)?;
    validate_skill_ids(skill_ids)?;

    let profile = ProfileFile {
        skill: skill_ids
            .iter()
            .map(|id| ProfileSkillEntry { id: id.clone() })
            .collect(),
    };
    write_profile(store, name, &profile)
}

pub fn ensure_profile(store: &StorePaths, name: &str) -> Result<(), SkmError> {
    store.ensure_initialized()?;
    validate_profile_name(name)?;
    if !store.profile_file(name).is_file() {
        create_profile(store, name, &[])?;
    }
    Ok(())
}

pub fn set_profile_skills(
    store: &StorePaths,
    name: &str,
    skill_ids: &[String],
) -> Result<(), SkmError> {
    store.ensure_initialized()?;
    validate_profile_name(name)?;
    let path = store.profile_file(name);
    if !path.is_file() {
        return Err(SkmError::ProfileNotFound(name.to_string()));
    }
    validate_skill_ids(skill_ids)?;

    let profile = ProfileFile {
        skill: skill_ids
            .iter()
            .map(|id| ProfileSkillEntry { id: id.clone() })
            .collect(),
    };
    write_profile(store, name, &profile)
}

pub fn list_profiles(store: &StorePaths) -> Result<Vec<String>, SkmError> {
    store.ensure_initialized()?;
    let dir = store.profiles_dir();
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

pub fn remove_profile(store: &StorePaths, name: &str) -> Result<(), SkmError> {
    store.ensure_initialized()?;
    validate_profile_name(name)?;
    let path = store.profile_file(name);
    if !path.is_file() {
        return Err(SkmError::ProfileNotFound(name.to_string()));
    }

    fs::remove_file(path)?;
    Ok(())
}

/// Refuse removal when `name` is active in project or user setup under `cwd`.
pub fn ensure_profile_not_active(cwd: &Path, name: &str) -> Result<(), SkmError> {
    use crate::config::{project_setup_path, read_setup, user_setup_path};

    for path in [project_setup_path(cwd), user_setup_path()] {
        if !path.is_file() {
            continue;
        }
        let setup = read_setup(&path)?;
        if setup.profile.active.as_deref() == Some(name) {
            return Err(SkmError::ActiveProfileRemoval(name.to_string()));
        }
    }
    Ok(())
}

pub fn read_profile(path: &Path) -> Result<ProfileFile, SkmError> {
    let content = fs::read_to_string(path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            SkmError::ProfileNotFound(
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            )
        } else {
            SkmError::Io(e)
        }
    })?;
    let profile: ProfileFile = toml::from_str(&content).map_err(|e| SkmError::InvalidProfile {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    let skill_ids: Vec<String> = profile.skill.iter().map(|entry| entry.id.clone()).collect();
    validate_skill_ids(&skill_ids)?;
    Ok(profile)
}

pub fn load_profile(store: &StorePaths, name: &str) -> Result<ProfileFile, SkmError> {
    validate_profile_name(name)?;
    let path = store.profile_file(name);
    if !path.is_file() {
        return Err(SkmError::ProfileNotFound(name.to_string()));
    }
    read_profile(&path)
}

pub fn write_profile(
    store: &StorePaths,
    name: &str,
    profile: &ProfileFile,
) -> Result<(), SkmError> {
    validate_profile_name(name)?;
    fs::create_dir_all(store.profiles_dir())?;
    let content = toml::to_string_pretty(profile)?;
    fs::write(store.profile_file(name), content)?;
    Ok(())
}

pub fn interactive_setup(store: &StorePaths, selected: &[String]) -> Result<Vec<String>, SkmError> {
    if !io::stdin().is_terminal() {
        return Err(SkmError::NotATty);
    }

    let disabled = read_disabled_ids(store)?;
    let mut pool = list_pool_ids(store)?;
    for id in selected {
        if disabled.contains(id) && !pool.contains(id) {
            pool.push(id.clone());
        }
    }
    pool.sort();
    pool.dedup();

    if pool.is_empty() {
        return Err(SkmError::EmptyPool);
    }

    let selected_set: std::collections::HashSet<&str> =
        selected.iter().map(String::as_str).collect();
    let labels: Vec<String> = pool
        .iter()
        .map(|id| {
            if disabled.contains(id) {
                format!("{id} (disabled)")
            } else {
                id.clone()
            }
        })
        .collect();
    let defaults: Vec<bool> = pool
        .iter()
        .map(|id| selected_set.contains(id.as_str()))
        .collect();

    let selection = MultiSelect::new()
        .with_prompt("Toggle skills (space to enable/disable, enter to confirm)")
        .items(&labels)
        .defaults(&defaults)
        .interact_opt()
        .map_err(|_| SkmError::SelectionCancelled)?;

    match selection {
        Some(indices) => Ok(indices.into_iter().map(|i| pool[i].clone()).collect()),
        None => Err(SkmError::SelectionCancelled),
    }
}

/// Remove matching skill IDs from every profile. Returns updated profile names.
pub fn remove_skills_from_profiles(
    store: &StorePaths,
    skill_ids: &[String],
) -> Result<Vec<String>, SkmError> {
    if skill_ids.is_empty() {
        return Ok(Vec::new());
    }

    let remove: std::collections::HashSet<&str> = skill_ids.iter().map(String::as_str).collect();
    let mut updated = Vec::new();

    for name in list_profiles(store)? {
        let profile = load_profile(store, &name)?;
        let remaining: Vec<String> = profile
            .skill
            .iter()
            .map(|entry| entry.id.clone())
            .filter(|id| !remove.contains(id.as_str()))
            .collect();

        if remaining.len() == profile.skill.len() {
            continue;
        }

        set_profile_skills(store, &name, &remaining)?;
        updated.push(name);
    }

    Ok(updated)
}

/// Profile names that reference any of the given skill IDs.
pub fn profiles_referencing_skills(
    store: &StorePaths,
    skill_ids: &[String],
) -> Result<Vec<String>, SkmError> {
    let skill_set: std::collections::HashSet<&str> = skill_ids.iter().map(String::as_str).collect();
    let mut names = Vec::new();
    for name in list_profiles(store)? {
        let profile = load_profile(store, &name)?;
        if profile
            .skill
            .iter()
            .any(|entry| skill_set.contains(entry.id.as_str()))
        {
            names.push(name);
        }
    }
    Ok(names)
}

fn validate_skill_ids(skill_ids: &[String]) -> Result<(), SkmError> {
    let mut seen = std::collections::HashSet::new();
    for id in skill_ids {
        validate_store_skill_id(id)?;
        if !seen.insert(id.clone()) {
            return Err(SkmError::DuplicateSkillId(id.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{init_store_layout, StorePaths};
    use tempfile::TempDir;

    #[test]
    fn roundtrip_profile() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        create_profile(&store, "infra", &["docx".to_string(), "git".to_string()]).unwrap();
        let profile = load_profile(&store, "infra").unwrap();
        assert_eq!(profile.skill.len(), 2);
        assert_eq!(profile.skill[0].id, "docx");
    }

    #[test]
    fn ensure_profile_creates_missing_profile() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        ensure_profile(&store, "infra").unwrap();
        let profile = load_profile(&store, "infra").unwrap();
        assert!(profile.skill.is_empty());
    }

    #[test]
    fn set_profile_skills_replaces_selection() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        create_profile(&store, "infra", &["docx".to_string(), "git".to_string()]).unwrap();
        set_profile_skills(&store, "infra", &["git".to_string(), "tf".to_string()]).unwrap();
        let profile = load_profile(&store, "infra").unwrap();
        assert_eq!(profile.skill.len(), 2);
        assert_eq!(profile.skill[0].id, "git");
        assert_eq!(profile.skill[1].id, "tf");
    }

    #[test]
    fn duplicate_ids_rejected() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let err =
            create_profile(&store, "infra", &["docx".to_string(), "docx".to_string()]).unwrap_err();
        assert!(matches!(err, SkmError::DuplicateSkillId(_)));
    }

    #[test]
    fn read_profile_rejects_invalid_skill_id() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        fs::write(
            store.profile_file("bad"),
            "[[skill]]\nid = \"INVALID ID\"\n",
        )
        .unwrap();

        let err = load_profile(&store, "bad").unwrap_err();
        assert!(matches!(err, SkmError::InvalidSkillId(_)));
    }
}
