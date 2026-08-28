use std::fs;
use std::path::{Path, PathBuf};

use crate::error::SkmError;
use crate::util::{discover_all_skill_dirs, path_to_store_skill_id};

pub mod pool;
pub mod profiles;
pub mod skills;
pub mod validate;

pub use validate::{inspect_store_path, prepare_store, validate_initialized_store, StoreState};

#[derive(Debug, Clone)]
pub struct StorePaths {
    root: PathBuf,
}

impl StorePaths {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn skm_dir(&self) -> PathBuf {
        self.root.join(".skm")
    }

    pub fn profiles_dir(&self) -> PathBuf {
        self.skm_dir().join("profiles")
    }

    pub fn meta_dir(&self) -> PathBuf {
        self.skm_dir().join("meta")
    }

    pub fn index_db(&self) -> PathBuf {
        self.skm_dir().join("index.db")
    }

    pub fn profile_file(&self, name: &str) -> PathBuf {
        self.profiles_dir().join(format!("{name}.toml"))
    }

    pub fn meta_file(&self, id: &str) -> PathBuf {
        let path = self.meta_dir().join(format!("{id}.toml"));
        path
    }

    pub fn skill_dir(&self, id: &str) -> PathBuf {
        id.split('/')
            .fold(self.root.clone(), |acc, seg| acc.join(seg))
    }

    pub fn disabled_file(&self) -> PathBuf {
        self.skm_dir().join("disabled.toml")
    }

    pub fn is_initialized(&self) -> bool {
        self.skm_dir().is_dir()
    }

    pub fn ensure_initialized(&self) -> Result<(), SkmError> {
        if !self.is_initialized() {
            return Err(SkmError::StoreNotInitialized);
        }
        Ok(())
    }

    pub fn canonical_root(&self) -> PathBuf {
        self.root()
            .canonicalize()
            .unwrap_or_else(|_| self.root().to_path_buf())
    }
}

pub fn init_store_layout(store: &StorePaths) -> Result<(), SkmError> {
    fs::create_dir_all(store.profiles_dir())?;
    fs::create_dir_all(store.meta_dir())?;
    let _ = crate::db::open_index(store)?;
    Ok(())
}

pub fn ensure_store_subdirs(store: &StorePaths) -> Result<(), SkmError> {
    if store.is_initialized() {
        fs::create_dir_all(store.profiles_dir())?;
        fs::create_dir_all(store.meta_dir())?;
    }
    Ok(())
}

/// Discover all skill directories under the store (e.g. `docx`, `engineering/tdd`).
pub fn discover_skill_ids(store: &StorePaths) -> Result<Vec<String>, SkmError> {
    let mut ids = Vec::new();
    if store.root().is_dir() {
        discover_skills_under(store.root(), store.root(), &mut ids)?;
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

fn discover_skills_under(
    store_root: &Path,
    dir: &Path,
    ids: &mut Vec<String>,
) -> Result<(), SkmError> {
    for skill_path in discover_all_skill_dirs(dir)? {
        if !skill_path.starts_with(store_root) {
            continue;
        }
        if let Some(id) = path_to_store_skill_id(store_root, &skill_path) {
            ids.push(id);
        }
    }
    Ok(())
}

/// Bundle root id for a nested skill id like `engineering/tdd` → `engineering`.
pub fn bundle_meta_for_skill(store: &StorePaths, skill_id: &str) -> Option<String> {
    let bundle_id = meta_owner_id(skill_id);
    if bundle_id == skill_id {
        return None;
    }
    if store.meta_file(bundle_id).is_file() {
        Some(bundle_id.to_string())
    } else {
        None
    }
}

/// Store-relative id used for provenance meta (`local/foo` → `local`).
pub fn meta_owner_id(skill_id: &str) -> &str {
    skill_id.split('/').next().unwrap_or(skill_id)
}

/// True when the skill has its own meta file or inherits bundle-level meta.
pub fn has_skill_meta(store: &StorePaths, skill_id: &str) -> bool {
    store.meta_file(skill_id).is_file() || bundle_meta_for_skill(store, skill_id).is_some()
}

/// Write provenance meta for on-disk skills that have none (create-if-missing only).
pub fn ensure_meta_for_discovered_skills(store: &StorePaths) -> Result<(), SkmError> {
    use chrono::Utc;

    use crate::config::SkillMeta;
    use crate::store::skills;
    use crate::util::{hash_directory, is_skill_dir};

    for id in discover_skill_ids(store)? {
        if skills::is_skill_disabled(store, &id)? {
            continue;
        }
        let skill_path = store.skill_dir(&id);
        if !is_skill_dir(&skill_path) || has_skill_meta(store, &id) {
            continue;
        }

        let meta_id = meta_owner_id(&id);
        let owner_path = store.skill_dir(meta_id);
        let meta = SkillMeta {
            source_type: "store".to_string(),
            path: owner_path.to_string_lossy().into_owned(),
            hash: hash_directory(&owner_path)?,
            imported_at: Utc::now().to_rfc3339(),
            transfer: "adopted".to_string(),
        };
        write_meta(store, meta_id, &toml::to_string_pretty(&meta)?)?;
    }
    Ok(())
}

/// List enabled library skill IDs (excludes disabled skills).
pub fn list_pool_ids(store: &StorePaths) -> Result<Vec<String>, SkmError> {
    skills::list_enabled_pool_ids(store)
}

fn write_meta_file(path: &Path, content: &str) -> Result<(), SkmError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

pub(crate) fn write_meta(store: &StorePaths, id: &str, content: &str) -> Result<(), SkmError> {
    write_meta_file(&store.meta_file(id), content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::init_store_layout;
    use tempfile::TempDir;

    #[test]
    fn discovers_nested_skills() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let eng = store.root().join("engineering");
        fs::create_dir_all(eng.join("tdd")).unwrap();
        fs::write(eng.join("tdd/SKILL.md"), "# tdd").unwrap();
        fs::write(eng.join("README.md"), "# eng").unwrap();

        let ids = discover_skill_ids(&store).unwrap();
        assert_eq!(ids, vec!["engineering/tdd"]);
    }

    #[test]
    fn discovers_deeply_nested_skills() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let skill = store.root().join("vendor/team/tdd");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "# tdd").unwrap();

        let ids = discover_skill_ids(&store).unwrap();
        assert_eq!(ids, vec!["vendor/team/tdd"]);
    }

    #[test]
    fn discovers_nested_skills_inside_skill_tree() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let parent = store.root().join("engineering/tdd");
        fs::create_dir_all(parent.join("advanced")).unwrap();
        fs::write(parent.join("SKILL.md"), "# tdd").unwrap();
        fs::write(parent.join("advanced/SKILL.md"), "# advanced tdd").unwrap();

        let ids = discover_skill_ids(&store).unwrap();
        assert_eq!(ids, vec!["engineering/tdd", "engineering/tdd/advanced"]);
    }
}
