mod exclude;
mod links;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::refresh_store_index;
use crate::error::SkmError;
use crate::progress;
use crate::progress::display_path;
use crate::resolver::{resolve, SkillPlacement};
use crate::setup::{select_command_setup, target_dirs_for_setup, SelectedSetup};
use crate::store::extends::load_flattened_profile;
use crate::store::skills::read_disabled_ids;
use crate::store::{ensure_store_subdirs, StorePaths};
use crate::util::{is_skill_dir, validate_profile_name};

pub(crate) use exclude::tracked_paths;
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

/// One target agent's view of the active profile: what is linked, and what a foreign entry
/// blocks. Reported per agent because the same profile can land differently in each directory.
#[derive(Debug, Clone)]
pub struct AgentStatus {
    pub agent: String,
    pub target: PathBuf,
    pub linked: Vec<PlacementStatus>,
    pub conflicts: Vec<PlacementConflict>,
}

pub fn reconcile(
    store: &StorePaths,
    cwd: &Path,
    force_user: bool,
    options: ReconcileOptions,
) -> Result<(), SkmError> {
    let selected = select_command_setup(cwd, force_user)?;
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
    let profile = load_flattened_profile(store, active)?;
    let disabled = read_disabled_ids(store)?;
    let placements = resolve(&profile, store, &disabled).map_err(SkmError::from)?;

    let targets = target_dirs_for_setup(selected)?;
    let store_root = store.canonical_root();
    // With one agent the diff lines speak for themselves; with several, the same skill name is
    // reported once per directory, so say which directory each run of lines belongs to.
    let announce_agents = targets.len() > 1;

    if options.dry_run {
        let mut excludes = Vec::with_capacity(targets.len());
        for target in &targets {
            if announce_agents {
                progress::step(format!("{}: {}", target.agent, display_path(&target.dir)));
            }
            let (to_wire, to_unwire, skipped) =
                compute_link_changes(&target.dir, &placements, &store_root)?;
            let skipped_set: HashSet<&str> = skipped.iter().map(String::as_str).collect();
            let names: Vec<String> = placements
                .iter()
                .filter(|placement| !skipped_set.contains(placement.name.as_str()))
                .map(|placement| placement.name.clone())
                .collect();
            for name in to_unwire {
                progress::unwired(&name, true);
            }
            for name in skipped {
                progress::skipped_conflict(&name, true);
            }
            for name in to_wire {
                progress::wired(&name, true);
            }
            excludes.push((target.dir.clone(), names));
        }
        exclude::sync_local_exclude(
            &selected.project_root,
            &excludes,
            selected.setup.placement.ignore_links,
            true,
        )?;
        return Ok(());
    }

    ensure_store_subdirs(store)?;
    progress::step("refreshing skill index");
    refresh_store_index(store)?;

    // Every directory is created and excluded before any link is written, so `git status` never
    // sees a store-owned link that the exclude block does not yet cover.
    let mut excludes = Vec::with_capacity(targets.len());
    for target in &targets {
        fs::create_dir_all(&target.dir)?;
        let wired_names: Vec<String> = placements
            .iter()
            .filter(|placement| {
                !is_foreign_occupant(&target.dir.join(&placement.name), &store_root)
            })
            .map(|placement| placement.name.clone())
            .collect();
        excludes.push((target.dir.clone(), wired_names));
    }
    exclude::sync_local_exclude(
        &selected.project_root,
        &excludes,
        selected.setup.placement.ignore_links,
        false,
    )?;

    let desired_names: HashSet<&str> = placements.iter().map(|p| p.name.as_str()).collect();

    for target in &targets {
        if announce_agents {
            progress::step(format!("{}: {}", target.agent, display_path(&target.dir)));
        }
        remove_dangling_store_symlinks(&target.dir, &store_root)?;
        clean_target(&target.dir, &store_root, &desired_names, false)?;
        prune_empty_skill_dirs(&target.dir)?;
        apply_placements(&target.dir, &placements, &store_root, false)?;
    }

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

/// Remove every store-owned symlink under `target` and prune emptied directories.
/// Used when leaving an agent's skills directory entirely (e.g. `switch-agent`), so
/// skm-managed links don't linger, orphaned and invisible, in the previous agent's dir.
pub fn unwire_all(target: &Path, store_root: &Path, dry_run: bool) -> Result<(), SkmError> {
    if !target.is_dir() {
        return Ok(());
    }
    walk_store_owned_symlinks(target, store_root, |path, rel| {
        progress::unwired(&rel, dry_run);
        if !dry_run {
            fs::remove_file(path)?;
        }
        Ok(())
    })?;
    if dry_run {
        return Ok(());
    }
    prune_empty_skill_dirs(target)
}

/// Drop the managed exclude block. Destroy uses this so leftover patterns cannot outlive
/// `.skm.toml`. `ignore_links = false` is the same rewrite.
pub fn clear_managed_exclude(project_root: &Path, dry_run: bool) -> Result<(), SkmError> {
    exclude::sync_local_exclude(project_root, &[], false, dry_run)
}

/// Rewrite the managed exclude block from the links that currently exist under the setup's
/// target directories.
///
/// Used when the agent set changes without a sync: paths belonging to a dropped agent have to
/// leave the block, while the agents that remain keep theirs. Basing it on the links actually
/// on disk means the block never claims to cover a link that was never written.
pub fn refresh_local_exclude(selected: &SelectedSetup, store_root: &Path) -> Result<(), SkmError> {
    let targets = target_dirs_for_setup(selected)?;
    let mut excludes = Vec::with_capacity(targets.len());
    for target in &targets {
        let mut names = Vec::new();
        if target.dir.is_dir() {
            walk_store_owned_symlinks(&target.dir, store_root, |_, rel| {
                names.push(rel);
                Ok(())
            })?;
        }
        excludes.push((target.dir.clone(), names));
    }
    exclude::sync_local_exclude(
        &selected.project_root,
        &excludes,
        selected.setup.placement.ignore_links,
        false,
    )
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
///
/// Does not walk into a skill root (`SKILL.md`): empty folders inside a project or
/// hand-installed skill are not leftover placements.
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
        if dir != root && is_skill_dir(dir) {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            // `file_type` does not follow symlinks, so a foreign symlink-to-dir is not entered.
            if entry.file_type()?.is_dir() {
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
) -> Result<Vec<AgentStatus>, SkmError> {
    let store_root = store.canonical_root();
    let targets = target_dirs_for_setup(selected)?;

    let mut reports: Vec<AgentStatus> = targets
        .into_iter()
        .map(|target| AgentStatus {
            agent: target.agent,
            target: target.dir,
            linked: Vec::new(),
            conflicts: Vec::new(),
        })
        .collect();

    let Some(active) = selected.setup.profile.active.as_deref() else {
        return Ok(reports);
    };

    let profile = load_flattened_profile(store, active)?;
    let disabled = read_disabled_ids(store)?;
    let placements = resolve(&profile, store, &disabled).map_err(SkmError::from)?;

    for report in &mut reports {
        for placement in &placements {
            let link_path = report.target.join(&placement.name);
            if is_foreign_occupant(&link_path, &store_root) {
                report.conflicts.push(PlacementConflict {
                    name: placement.name.clone(),
                    store_id: placement.store_id.clone(),
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
                .unwrap_or_else(|_| placement.source.clone());
            if current == want {
                report.linked.push(PlacementStatus {
                    name: placement.name.clone(),
                    source: current,
                });
            }
        }

        report.linked.sort_by(|a, b| a.name.cmp(&b.name));
        report.conflicts.sort_by(|a, b| a.name.cmp(&b.name));
    }

    Ok(reports)
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

    #[cfg(unix)]
    #[test]
    fn apply_placements_skips_foreign_dangling_symlink_instead_of_erroring() {
        let root = TempDir::new().unwrap();
        let store_root = root.path().join("store");
        fs::create_dir_all(&store_root).unwrap();
        let docx_src = store_root.join("docx");
        fs::create_dir_all(&docx_src).unwrap();

        let target = root.path().join("agent");
        fs::create_dir_all(&target).unwrap();
        // A hand-created broken symlink occupies the placement name.
        std::os::unix::fs::symlink(root.path().join("does-not-exist"), target.join("docx"))
            .unwrap();

        let placements = vec![SkillPlacement {
            store_id: "docx".into(),
            name: "docx".into(),
            source: docx_src,
        }];

        let store_canon = store_root.canonicalize().unwrap();
        let result = apply_placements(&target, &placements, &store_canon, false);
        assert!(
            result.is_ok(),
            "a foreign dangling symlink at the placement name should be skipped as a conflict, not error: {result:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn unwire_all_does_not_prune_empty_dirs_inside_a_project_skill() {
        let root = TempDir::new().unwrap();
        let store_root = root.path().join("store");
        let skill = store_root.join("docx");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "# docx\n").unwrap();

        let target = root.path().join("agent");
        let project = target.join("project-skill");
        fs::create_dir_all(project.join("scripts")).unwrap();
        fs::write(project.join("SKILL.md"), "# project\n").unwrap();
        std::os::unix::fs::symlink(&skill, target.join("docx")).unwrap();

        let store_canon = store_root.canonicalize().unwrap();
        unwire_all(&target, &store_canon, false).unwrap();

        assert!(!target.join("docx").exists());
        assert!(project.join("SKILL.md").is_file());
        assert!(project.join("scripts").is_dir());
    }

    #[test]
    fn unwire_all_prunes_empty_nested_placement_dirs() {
        let root = TempDir::new().unwrap();
        let store_root = root.path().join("store");
        fs::create_dir_all(&store_root).unwrap();
        let target = root.path().join("agent");
        fs::create_dir_all(target.join("engineering")).unwrap();

        unwire_all(&target, &store_root, false).unwrap();

        assert!(!target.join("engineering").exists());
        assert!(target.is_dir());
    }
}
