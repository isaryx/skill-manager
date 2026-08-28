mod links;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::refresh_store_index;
use crate::error::SkmError;
use crate::progress;
use crate::resolver::{resolve, SkillPlacement};
use crate::setup::{select_setup, target_dir_for_setup, SelectedSetup};
use crate::store::profiles::load_profile;
use crate::store::skills::read_disabled_ids;
use crate::store::{ensure_store_subdirs, StorePaths};
use crate::util::validate_profile_name;

pub(crate) use links::{
    is_foreign_occupant, is_store_owned_symlink, resolve_symlink_target as resolve_link_target,
    walk_store_owned_symlinks,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct ReconcileOptions {
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct PlacementStatus {
    pub name: String,
    pub source: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PlacementConflict {
    pub name: String,
    pub store_id: String,
}

#[derive(Debug, Clone)]
pub struct StatusReport {
    pub linked: Vec<PlacementStatus>,
    pub conflicts: Vec<PlacementConflict>,
}

pub fn reconcile(
    store: &StorePaths,
    cwd: &Path,
    force_user: bool,
    options: ReconcileOptions,
) -> Result<(), SkmError> {
    let selected = select_setup(cwd, force_user)?;
    reconcile_with_setup(store, &selected, None, options)
}

/// Sync skill links for `profile` without saving it as the active profile.
pub fn reconcile_for_profile(
    store: &StorePaths,
    selected: &SelectedSetup,
    profile: &str,
    options: ReconcileOptions,
) -> Result<(), SkmError> {
    validate_profile_name(profile)?;
    reconcile_with_setup(store, selected, Some(profile), options)
}

pub fn reconcile_with_setup(
    store: &StorePaths,
    selected: &SelectedSetup,
    profile_override: Option<&str>,
    options: ReconcileOptions,
) -> Result<(), SkmError> {
    if !store.is_initialized() {
        return Err(SkmError::StoreNotInitialized);
    }

    let active = profile_override.map(Ok).unwrap_or_else(|| {
        selected
            .setup
            .profile
            .active
            .as_deref()
            .ok_or(SkmError::NoActiveProfile)
    })?;

    progress::step(format!("loading profile `{active}`"));
    let profile = load_profile(store, active)?;
    let disabled = read_disabled_ids(store)?;
    let placements = resolve(&profile, store, &disabled).map_err(SkmError::from)?;

    let (_agent, target) = target_dir_for_setup(selected)?;
    let store_root = store.canonical_root();

    if options.dry_run {
        let (to_wire, to_unwire, skipped) =
            compute_link_changes(&target, &placements, &store_root)?;
        for name in to_unwire {
            progress::unwired(&name, true);
        }
        for name in skipped {
            progress::skipped_conflict(&name, true);
        }
        for name in to_wire {
            progress::wired(&name, true);
        }
        return Ok(());
    }

    ensure_store_subdirs(store)?;
    progress::step("refreshing skill index");
    refresh_store_index(store)?;

    fs::create_dir_all(&target)?;

    remove_dangling_store_symlinks(&target, &store_root)?;

    let desired_names: HashSet<&str> = placements.iter().map(|p| p.name.as_str()).collect();

    clean_target(&target, &store_root, &desired_names, false)?;
    prune_empty_skill_dirs(&target)?;
    apply_placements(&target, &placements, &store_root, false)?;

    Ok(())
}

/// `(to_wire, to_unwire, skipped_conflicts)`
type LinkChangeDiff = (Vec<String>, Vec<String>, Vec<String>);

/// Diff store-owned symlinks in `target` against resolved `placements`.
/// Returns `(to_wire, to_unwire, skipped_conflicts)`.
pub fn compute_link_changes(
    target: &Path,
    placements: &[SkillPlacement],
    store_root: &Path,
) -> Result<LinkChangeDiff, SkmError> {
    let mut current: HashMap<String, PathBuf> = HashMap::new();
    if target.is_dir() {
        walk_store_owned_symlinks(target, store_root, |path, rel| {
            if let Some(src) = resolve_link_target(path) {
                current.insert(rel, src);
            }
            Ok(())
        })?;
    }

    let mut desired: HashMap<String, PathBuf> = HashMap::new();
    let mut skipped = Vec::new();
    for placement in placements {
        let link_path = target.join(&placement.name);
        if is_foreign_occupant(&link_path, store_root) {
            skipped.push(placement.name.clone());
            continue;
        }
        let source = placement
            .source
            .canonicalize()
            .unwrap_or_else(|_| placement.source.clone());
        desired.insert(placement.name.clone(), source);
    }

    let mut to_unwire = Vec::new();
    let mut to_wire = Vec::new();

    for (name, current_src) in &current {
        match desired.get(name) {
            None => to_unwire.push(name.clone()),
            Some(want) if current_src != want => {
                to_unwire.push(name.clone());
                to_wire.push(name.clone());
            }
            _ => {}
        }
    }

    for name in desired.keys() {
        if !current.contains_key(name) {
            to_wire.push(name.clone());
        }
    }

    to_unwire.sort();
    to_wire.sort();
    skipped.sort();
    to_unwire.dedup();
    to_wire.dedup();
    skipped.dedup();
    Ok((to_wire, to_unwire, skipped))
}

fn remove_dangling_store_symlinks(target: &Path, store_root: &Path) -> Result<(), SkmError> {
    walk_store_owned_symlinks(target, store_root, |path, _| {
        if resolve_link_target(path).is_none() {
            fs::remove_file(path)?;
        }
        Ok(())
    })
}

fn clean_target(
    target: &Path,
    store_root: &Path,
    desired: &HashSet<&str>,
    dry_run: bool,
) -> Result<(), SkmError> {
    walk_store_owned_symlinks(target, store_root, |path, rel| {
        if !desired.contains(rel.as_str()) {
            progress::unwired(&rel, dry_run);
            if !dry_run {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    })
}

/// Remove empty directories left behind after cleaning legacy nested placements.
fn prune_empty_skill_dirs(target: &Path) -> Result<(), SkmError> {
    fn prune(dir: &Path, root: &Path) -> Result<(), SkmError> {
        if !dir.is_dir() {
            return Ok(());
        }
        if dir != root
            && fs::symlink_metadata(dir)
                .map(|meta| meta.file_type().is_symlink())
                .unwrap_or(false)
        {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                prune(&path, root)?;
            }
        }

        if dir != root && fs::read_dir(dir)?.next().is_none() {
            fs::remove_dir(dir)?;
        }
        Ok(())
    }

    prune(target, target)
}

fn apply_placements(
    target: &Path,
    placements: &[SkillPlacement],
    store_root: &Path,
    dry_run: bool,
) -> Result<(), SkmError> {
    if !dry_run {
        fs::create_dir_all(target)?;
    }
    for placement in placements {
        let link_path = target.join(&placement.name);
        if is_foreign_occupant(&link_path, store_root) {
            progress::skipped_conflict(&placement.name, dry_run);
            continue;
        }

        let source = placement
            .source
            .canonicalize()
            .unwrap_or(placement.source.clone());

        if link_path.exists() && is_store_owned_symlink(&link_path, store_root) {
            if let Some(current) = resolve_link_target(&link_path) {
                if current == source {
                    continue;
                }
            }
            progress::unwired(&placement.name, dry_run);
            if !dry_run {
                fs::remove_file(&link_path)?;
            }
        }

        if dry_run {
            progress::wired(&placement.name, true);
            continue;
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&source, &link_path)?;
            progress::wired(&placement.name, false);
        }
        #[cfg(not(unix))]
        {
            return Err(SkmError::Usage(
                "symlinks are not supported on this platform".to_string(),
            ));
        }
    }
    Ok(())
}

pub fn collect_status(
    store: &StorePaths,
    selected: &SelectedSetup,
) -> Result<StatusReport, SkmError> {
    let store_root = store.canonical_root();
    let (_agent, target) = target_dir_for_setup(selected)?;

    let mut linked = Vec::new();
    let mut conflicts = Vec::new();

    let Some(active) = selected.setup.profile.active.as_deref() else {
        return Ok(StatusReport { linked, conflicts });
    };

    let profile = load_profile(store, active)?;
    let disabled = read_disabled_ids(store)?;
    let placements = resolve(&profile, store, &disabled).map_err(SkmError::from)?;

    for placement in placements {
        let link_path = target.join(&placement.name);
        if is_foreign_occupant(&link_path, &store_root) {
            conflicts.push(PlacementConflict {
                name: placement.name.clone(),
                store_id: placement.store_id,
            });
            continue;
        }

        if !is_store_owned_symlink(&link_path, &store_root) {
            continue;
        }

        let Some(current) = resolve_link_target(&link_path) else {
            continue;
        };
        let want = placement
            .source
            .canonicalize()
            .unwrap_or(placement.source);
        if current == want {
            linked.push(PlacementStatus {
                name: placement.name,
                source: current,
            });
        }
    }

    linked.sort_by(|a, b| a.name.cmp(&b.name));
    conflicts.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(StatusReport { linked, conflicts })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::resolver::SkillPlacement;
    use tempfile::TempDir;

    #[test]
    fn compute_link_changes_skips_foreign_occupant() {
        let root = TempDir::new().unwrap();
        let store_root = root.path().join("store");
        fs::create_dir_all(&store_root).unwrap();
        let docx_src = store_root.join("docx");
        fs::create_dir_all(&docx_src).unwrap();
        let other_src = store_root.join("other");
        fs::create_dir_all(&other_src).unwrap();

        let target = root.path().join("agent");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("docx"), "blocked").unwrap();

        let placements = vec![
            SkillPlacement {
                store_id: "docx".into(),
                name: "docx".into(),
                source: docx_src,
            },
            SkillPlacement {
                store_id: "other".into(),
                name: "other".into(),
                source: other_src,
            },
        ];

        let store_canon = store_root.canonicalize().unwrap();
        let (to_wire, to_unwire, skipped) =
            compute_link_changes(&target, &placements, &store_canon).unwrap();

        assert_eq!(skipped, vec!["docx"]);
        assert!(!to_wire.contains(&"docx".to_string()));
        assert!(to_wire.contains(&"other".to_string()));
        assert!(to_unwire.is_empty());
    }
}
