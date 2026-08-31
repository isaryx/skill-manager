use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::config::SkillMeta;
use crate::db::rebuild_from_store;
use crate::error::SkmError;
use crate::store::{discover_skill_ids, write_meta, StorePaths};
use crate::util::{
    copy_dir_all, hash_directory, is_skill_dir, is_skill_tree, validate_store_entry_name,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    Copy,
    Move,
}

pub fn add_skill(
    store: &StorePaths,
    source: &Path,
    mode: TransferMode,
    as_name: Option<&str>,
) -> Result<String, SkmError> {
    store.ensure_initialized()?;

    if !is_skill_dir(source) {
        return Err(SkmError::NotASkillDir(source.to_path_buf()));
    }

    let name = match as_name {
        Some(n) => n.to_string(),
        None => source
            .file_name()
            .ok_or_else(|| SkmError::NotASkillDir(source.to_path_buf()))?
            .to_string_lossy()
            .into_owned(),
    };

    validate_store_entry_name(&name)?;

    let dest = store.skill_dir(&name);
    if dest.exists() {
        return Err(SkmError::DestinationExists(dest));
    }

    let source_path = fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());

    match mode {
        TransferMode::Copy => copy_dir_all(source, &dest)?,
        TransferMode::Move => fs::rename(source, &dest)?,
    }

    write_skill_meta(store, &name, &source_path, &dest, mode)?;
    rebuild_from_store(store)?;
    Ok(name)
}

/// Import a skill bundle as one tree under `$SKM_STORE/<name>/` (preserves parent + children).
pub fn add_skill_tree(
    store: &StorePaths,
    source: &Path,
    mode: TransferMode,
    as_name: Option<&str>,
) -> Result<Vec<String>, SkmError> {
    store.ensure_initialized()?;

    if !is_skill_tree(source) {
        return Err(SkmError::EmptySkillBundle(source.to_path_buf()));
    }

    let name = match as_name {
        Some(n) => n.to_string(),
        None => source
            .file_name()
            .ok_or_else(|| SkmError::NotASkillDir(source.to_path_buf()))?
            .to_string_lossy()
            .into_owned(),
    };

    validate_store_entry_name(&name)?;

    let dest = store.skill_dir(&name);
    if dest.exists() {
        return Err(SkmError::DestinationExists(dest));
    }

    let source_path = fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());

    match mode {
        TransferMode::Copy => copy_dir_all(source, &dest)?,
        TransferMode::Move => fs::rename(source, &dest)?,
    }

    let meta = SkillMeta {
        source_type: "local-bundle".to_string(),
        path: source_path.to_string_lossy().into_owned(),
        hash: hash_directory(&dest)?,
        imported_at: Utc::now().to_rfc3339(),
        transfer: match mode {
            TransferMode::Copy => "copy",
            TransferMode::Move => "move",
        }
        .to_string(),
    };
    let meta_content = toml::to_string_pretty(&meta)?;
    write_meta(store, &name, &meta_content)?;

    rebuild_from_store(store)?;

    let prefix = format!("{name}/");
    let skill_ids: Vec<String> = discover_skill_ids(store)?
        .into_iter()
        .filter(|id| id == &name || id.starts_with(&prefix))
        .collect();

    if skill_ids.is_empty() {
        return Err(SkmError::EmptySkillBundle(source.to_path_buf()));
    }

    Ok(skill_ids)
}

fn write_skill_meta(
    store: &StorePaths,
    name: &str,
    source_path: &Path,
    dest: &Path,
    mode: TransferMode,
) -> Result<(), SkmError> {
    let meta = SkillMeta {
        source_type: "local".to_string(),
        path: source_path.to_string_lossy().into_owned(),
        hash: hash_directory(dest)?,
        imported_at: Utc::now().to_rfc3339(),
        transfer: match mode {
            TransferMode::Copy => "copy",
            TransferMode::Move => "move",
        }
        .to_string(),
    };
    let meta_content = toml::to_string_pretty(&meta)?;
    write_meta(store, name, &meta_content)
}

#[derive(Debug, Clone)]
pub struct SkillRemoval {
    pub id: String,
    pub skill_ids: Vec<String>,
    pub remove_root: PathBuf,
}

pub fn plan_skill_removal(store: &StorePaths, id: &str) -> Result<SkillRemoval, SkmError> {
    store.ensure_initialized()?;
    crate::util::validate_store_skill_id(id)?;

    let all = discover_skill_ids(store)?;
    let prefix = format!("{id}/");
    let skill_ids: Vec<String> = all
        .iter()
        .filter(|skill_id| skill_id.as_str() == id || skill_id.starts_with(&prefix))
        .cloned()
        .collect();

    let remove_root = store.skill_dir(id);
    if skill_ids.is_empty() && !remove_root.is_dir() {
        return Err(SkmError::SkillNotFound(id.to_string()));
    }

    Ok(SkillRemoval {
        id: id.to_string(),
        skill_ids,
        remove_root,
    })
}

pub fn remove_skill(store: &StorePaths, plan: &SkillRemoval) -> Result<(), SkmError> {
    store.ensure_initialized()?;

    if plan.remove_root.is_dir() {
        fs::remove_dir_all(&plan.remove_root)?;
        prune_empty_parents(store.root(), &plan.remove_root);
    }

    for skill_id in &plan.skill_ids {
        let meta = store.meta_file(skill_id);
        if meta.is_file() {
            fs::remove_file(meta)?;
        }
    }
    let bundle_meta = store.meta_file(&plan.id);
    if bundle_meta.is_file() {
        fs::remove_file(bundle_meta)?;
    }

    let skill_set: std::collections::HashSet<&str> =
        plan.skill_ids.iter().map(String::as_str).collect();
    let prefix = format!("{}/", plan.id);
    let disabled = crate::store::skills::read_disabled_ids(store)?;
    let remaining: Vec<String> = disabled
        .into_iter()
        .filter(|disabled_id| {
            !skill_set.contains(disabled_id.as_str()) && !disabled_id.starts_with(&prefix)
        })
        .collect();
    crate::store::skills::write_disabled_ids(store, &remaining)?;

    rebuild_from_store(store)?;
    Ok(())
}

fn prune_empty_parents(store_root: &Path, removed: &Path) {
    let mut dir = removed.parent();
    while let Some(current) = dir {
        if current == store_root {
            break;
        }
        let is_empty = fs::read_dir(current)
            .ok()
            .is_some_and(|mut entries| entries.next().is_none());
        if is_empty {
            let _ = fs::remove_dir(current);
            dir = current.parent();
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SkmError;
    use crate::store::{init_store_layout, StorePaths};
    use tempfile::TempDir;

    #[test]
    fn add_skill_tree_reports_root_skill_when_root_itself_is_a_skill() {
        let store_tmp = TempDir::new().unwrap();
        let store = StorePaths::new(store_tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let src_tmp = TempDir::new().unwrap();
        let src = src_tmp.path().join("parent");
        fs::create_dir_all(src.join("child")).unwrap();
        fs::write(src.join("SKILL.md"), "# parent").unwrap();
        fs::write(src.join("child/SKILL.md"), "# child").unwrap();

        let ids = add_skill_tree(&store, &src, TransferMode::Copy, None).unwrap();

        assert!(
            ids.contains(&"parent".to_string()),
            "expected the bundle root's own skill id to be reported alongside its nested child, got {ids:?}"
        );
        assert!(ids.contains(&"parent/child".to_string()));
    }

    #[test]
    fn add_skill_rejects_existing_destination() {
        let store_tmp = TempDir::new().unwrap();
        let store = StorePaths::new(store_tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let src_tmp = TempDir::new().unwrap();
        let src = src_tmp.path().join("demo");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("SKILL.md"), "# demo").unwrap();

        add_skill(&store, &src, TransferMode::Copy, None).unwrap();
        let err = add_skill(&store, &src, TransferMode::Copy, None).unwrap_err();
        assert!(matches!(err, SkmError::DestinationExists(_)));
    }
}
