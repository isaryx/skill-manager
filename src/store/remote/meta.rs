use std::path::Path;

use chrono::Utc;

use crate::config::{RepoRegistration, SkillMeta};
use crate::error::SkmError;
use crate::store::remote::discover::{find_skills_root, list_repo_skills};
use crate::store::remote::git::path_last_commit;
use crate::store::remote::paths::checkout_path;
use crate::store::remote::registry::list_repos;
use crate::store::{read_skill_meta, remove_skill_meta, write_meta, StorePaths};
use crate::util::hash_directory;

/// How aggressively to refresh per-skill remote meta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteSyncRecord {
    /// Re-read commit, hash, and synced_at (after pull, add, or pin).
    Full,
    /// After agent reconcile: bump synced_at when commit is unchanged; full write when it drifted.
    AfterReconcile,
}

/// Write per-skill meta (`commit`, `synced_at`) for skills in a remote repository.
pub fn record_remote_skill_sync(
    store: &StorePaths,
    reg: &RepoRegistration,
    skills: &[(String, std::path::PathBuf)],
    checkout: &Path,
    mode: RemoteSyncRecord,
) -> Result<(), SkmError> {
    let synced_at = Utc::now().to_rfc3339();
    for (id, skill_path) in skills {
        let commit = path_last_commit(checkout, skill_path)?;
        if mode == RemoteSyncRecord::AfterReconcile {
            if let Some(existing) = read_skill_meta(store, id)? {
                if existing.source_type == "remote" && existing.commit.as_deref() == Some(&commit) {
                    let mut updated = existing;
                    updated.synced_at = Some(synced_at.clone());
                    write_meta(store, id, &toml::to_string_pretty(&updated)?)?;
                    continue;
                }
            }
        }

        let imported_at = read_skill_meta(store, id)?
            .map(|m| m.imported_at)
            .unwrap_or_else(|| synced_at.clone());
        let meta = SkillMeta {
            source_type: "remote".to_string(),
            path: skill_path.to_string_lossy().into_owned(),
            hash: hash_directory(skill_path)?,
            imported_at,
            transfer: "clone".to_string(),
            repo_name: Some(reg.name.clone()),
            remote_url: Some(reg.url.clone()),
            commit: Some(commit),
            synced_at: Some(synced_at.clone()),
        };
        write_meta(store, id, &toml::to_string_pretty(&meta)?)?;
    }
    Ok(())
}

/// Record sync metadata for every skill in every registered remote checkout.
pub fn record_all_remote_skill_syncs(
    store: &StorePaths,
    mode: RemoteSyncRecord,
) -> Result<(), SkmError> {
    for reg in list_repos(store)? {
        let checkout = checkout_path(store, &reg.name);
        if !checkout.is_dir() {
            continue;
        }
        let skills_root = match find_skills_root(&checkout)? {
            Some(root) => root,
            None => continue,
        };
        let skills = list_repo_skills(&checkout, &skills_root, &reg.name)?;
        record_remote_skill_sync(store, &reg, &skills, &checkout, mode)?;
    }
    Ok(())
}

pub fn remove_remote_skill_meta(store: &StorePaths, skill_id: &str) -> Result<(), SkmError> {
    remove_skill_meta(store, skill_id)
}
