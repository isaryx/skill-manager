use std::collections::HashSet;
use std::fs;
use std::io::{self, IsTerminal};

use serde::{Deserialize, Serialize};

use crate::error::SkmError;
use crate::store::{discover_skill_ids, StorePaths};
use crate::tui::{MultiSelect, MultiSelectItem};

const DISABLED_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
struct DisabledSkillsFile {
    version: u32,
    #[serde(default)]
    ids: Vec<String>,
}

pub fn read_disabled_ids(store: &StorePaths) -> Result<HashSet<String>, SkmError> {
    store.ensure_initialized()?;
    let path = store.disabled_file();
    if !path.is_file() {
        return Ok(HashSet::new());
    }

    let content = fs::read_to_string(&path)?;
    let file: DisabledSkillsFile =
        toml::from_str(&content).map_err(|e| SkmError::InvalidStore {
            path: path.clone(),
            message: format!("invalid disabled skills file: {e}"),
        })?;

    Ok(file.ids.into_iter().collect())
}

pub fn write_disabled_ids(store: &StorePaths, ids: &[String]) -> Result<(), SkmError> {
    store.ensure_initialized()?;
    let mut unique: Vec<String> = ids.to_vec();
    unique.sort();
    unique.dedup();

    let path = store.disabled_file();
    if unique.is_empty() {
        if path.is_file() {
            fs::remove_file(path)?;
        }
        return Ok(());
    }

    let file = DisabledSkillsFile {
        version: DISABLED_VERSION,
        ids: unique,
    };
    fs::create_dir_all(store.skm_dir())?;
    fs::write(path, toml::to_string_pretty(&file)?)?;
    Ok(())
}

pub fn list_enabled_pool_ids(store: &StorePaths) -> Result<Vec<String>, SkmError> {
    store.ensure_initialized()?;
    let disabled = read_disabled_ids(store)?;
    Ok(discover_skill_ids(store)?
        .into_iter()
        .filter(|id| !disabled.contains(id))
        .collect())
}

/// Pick which library skills stay enabled, in a full-screen list. Checked means enabled.
pub fn interactive_skills_setup(store: &StorePaths) -> Result<(), SkmError> {
    if !io::stdin().is_terminal() {
        return Err(SkmError::NotATty);
    }

    let pool = discover_skill_ids(store)?;
    if pool.is_empty() {
        return Err(SkmError::EmptyPool);
    }

    let disabled = read_disabled_ids(store)?;
    let items = pool
        .iter()
        .map(|id| MultiSelectItem::new(id).selected(!disabled.contains(id)));

    let enabled: HashSet<String> = MultiSelect::new("Skills enabled in the library")
        .items(items)
        .interact()?
        .into_iter()
        .collect();

    let new_disabled: Vec<String> = pool
        .into_iter()
        .filter(|id| !enabled.contains(id))
        .collect();
    write_disabled_ids(store, &new_disabled)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::init_store_layout;
    use std::fs;
    use tempfile::TempDir;

    fn store_with_skills(ids: &[&str]) -> (TempDir, StorePaths) {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();
        for id in ids {
            let dir = store.skill_dir(id);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("SKILL.md"), "# test").unwrap();
        }
        (tmp, store)
    }

    #[test]
    fn empty_disabled_file_means_all_enabled() {
        let (_tmp, store) = store_with_skills(&["docx", "git"]);
        let enabled = list_enabled_pool_ids(&store).unwrap();
        assert_eq!(enabled, vec!["docx", "git"]);
    }

    #[test]
    fn disabled_skills_excluded_from_enabled_pool() {
        let (_tmp, store) = store_with_skills(&["docx", "git"]);
        write_disabled_ids(&store, &["docx".to_string()]).unwrap();
        let enabled = list_enabled_pool_ids(&store).unwrap();
        assert_eq!(enabled, vec!["git"]);
    }

    #[test]
    fn clearing_disabled_removes_file() {
        let (_tmp, store) = store_with_skills(&["docx"]);
        write_disabled_ids(&store, &["docx".to_string()]).unwrap();
        assert!(store.disabled_file().is_file());
        write_disabled_ids(&store, &[]).unwrap();
        assert!(!store.disabled_file().is_file());
    }
}
