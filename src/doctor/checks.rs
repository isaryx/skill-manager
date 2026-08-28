use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::adapters::get_adapter;
use crate::db::{list_skills, open_index};
use crate::error::SkmError;
use crate::progress::display_path;
use crate::resolver::resolve;
use crate::setup::{target_dir_for_setup, SelectedSetup};
use crate::store::profiles::{list_profiles, load_profile};
use crate::store::skills::{list_enabled_pool_ids, read_disabled_ids};
use crate::store::validate::{inspect_store_path, StoreState};
use crate::store::{discover_skill_ids, has_skill_meta, StorePaths};
use crate::sync::{is_foreign_occupant, resolve_link_target, walk_store_owned_symlinks};
use crate::util::{is_path_inside, is_skill_bundle, is_skill_dir, path_to_store_skill_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warn,
    Info,
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl Issue {
    fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            profile: None,
            skill: None,
            path: None,
        }
    }

    fn warn(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Warn,
            message: message.into(),
            profile: None,
            skill: None,
            path: None,
        }
    }

    fn info(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Info,
            message: message.into(),
            profile: None,
            skill: None,
            path: None,
        }
    }

    fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    fn with_skill(mut self, skill: impl Into<String>) -> Self {
        self.skill = Some(skill.into());
        self
    }

    fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
}

pub fn check_store(store: &StorePaths) -> Vec<Issue> {
    match inspect_store_path(store.root()) {
        Ok(StoreState::Absent) => vec![Issue::error(
            "store.missing",
            "skill store is not initialized; run `skm init`",
        )],
        Ok(StoreState::Invalid(message)) => vec![Issue::error(
            "store.invalid",
            format!(
                "invalid skill store at {}: {message}",
                store.root().display()
            ),
        )
        .with_path(display_path(store.root()))],
        Ok(StoreState::Valid) => Vec::new(),
        Err(err) => vec![Issue::error("store.invalid", err.to_string())],
    }
}

pub fn check_index(store: &StorePaths) -> Result<Vec<Issue>, SkmError> {
    let mut issues = Vec::new();

    let enabled_on_disk = list_enabled_pool_ids(store)?;
    let index_count = list_skills(&open_index(store)?).map(|rows| rows.len())?;

    if index_count != enabled_on_disk.len() {
        issues.push(Issue::warn(
            "index.stale",
            format!(
                "index lists {index_count} skills; {} found on disk (run `skm scan`)",
                enabled_on_disk.len()
            ),
        ));
    }

    Ok(issues)
}

pub fn check_skills_on_disk(store: &StorePaths) -> Result<Vec<Issue>, SkmError> {
    let mut issues = Vec::new();
    let store_root = store.root();

    if !store_root.is_dir() {
        return Ok(issues);
    }

    walk_candidate_skill_dirs(store_root, store_root, &mut issues)?;
    Ok(issues)
}

fn walk_candidate_skill_dirs(
    store_root: &Path,
    dir: &Path,
    issues: &mut Vec<Issue>,
) -> Result<(), SkmError> {
    if dir.file_name().and_then(|n| n.to_str()) == Some(".skm") {
        return Ok(());
    }

    // Subdirectories inside a valid skill (e.g. agents/, scripts/) are not skill roots.
    if dir != store_root && is_skill_dir(dir) {
        return Ok(());
    }

    if dir != store_root
        && dir.is_dir()
        && !is_skill_dir(dir)
        && !is_skill_bundle(dir)
        && dir_has_content(dir)?
    {
        if let Some(id) = path_to_store_skill_id(store_root, dir) {
            issues.push(
                Issue::warn(
                    "skill.missing_skill_md",
                    format!("skill directory `{id}` has no SKILL.md"),
                )
                .with_skill(id)
                .with_path(display_path(dir)),
            );
        }
    }

    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            walk_candidate_skill_dirs(store_root, &entry.path(), issues)?;
        }
    }

    Ok(())
}

fn dir_has_content(dir: &Path) -> Result<bool, SkmError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".skm" {
            continue;
        }
        return Ok(true);
    }
    Ok(false)
}

/// Meta is known when it matches a skill id or a bundle root for nested skills.
fn is_known_meta(meta_id: &str, skill_ids: &HashSet<String>) -> bool {
    if skill_ids.contains(meta_id) {
        return true;
    }
    let prefix = format!("{meta_id}/");
    skill_ids.iter().any(|id| id.starts_with(&prefix))
}

pub fn check_meta(store: &StorePaths) -> Result<Vec<Issue>, SkmError> {
    let mut issues = Vec::new();
    let skill_ids: HashSet<String> = discover_skill_ids(store)?.into_iter().collect();

    for id in &skill_ids {
        if !has_skill_meta(store, id) {
            issues.push(
                Issue::info(
                    "meta.missing",
                    format!(
                        "skill `{id}` has no metadata file (run `skm scan` to adopt on-disk skills)"
                    ),
                )
                .with_skill(id.clone()),
            );
        }
    }

    let meta_dir = store.meta_dir();
    if meta_dir.is_dir() {
        for entry in fs::read_dir(&meta_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if is_known_meta(stem, &skill_ids) {
                continue;
            }
            issues.push(
                Issue::warn(
                    "meta.orphan",
                    format!("metadata file `{stem}.toml` has no matching skill directory"),
                )
                .with_skill(stem.to_string())
                .with_path(display_path(&path)),
            );
        }
    }

    Ok(issues)
}

pub fn check_profiles(store: &StorePaths) -> Result<Vec<Issue>, SkmError> {
    let mut issues = Vec::new();
    let on_disk: HashSet<String> = discover_skill_ids(store)?.into_iter().collect();
    let disabled = read_disabled_ids(store)?;

    for name in list_profiles(store)? {
        let profile = load_profile(store, &name)?;

        if profile.skill.is_empty() {
            issues.push(
                Issue::warn(
                    "profile.empty",
                    format!("profile `{name}` contains no skills"),
                )
                .with_profile(&name),
            );
        }

        for entry in &profile.skill {
            let id = &entry.id;
            if !on_disk.contains(id) {
                issues.push(
                    Issue::error(
                        "profile.missing_ref",
                        format!("profile `{name}` references missing skill `{id}`"),
                    )
                    .with_profile(&name)
                    .with_skill(id.clone()),
                );
            } else if disabled.contains(id) {
                issues.push(
                    Issue::info(
                        "profile.disabled_ref",
                        format!("profile `{name}` includes disabled skill `{id}`"),
                    )
                    .with_profile(&name)
                    .with_skill(id.clone()),
                );
            }
        }
    }

    Ok(issues)
}

pub fn check_config(selected: &SelectedSetup) -> Vec<Issue> {
    let mut issues = Vec::new();

    if get_adapter(&selected.setup.placement.agent).is_err() {
        issues.push(Issue::error(
            "config.unknown_agent",
            format!(
                "unknown agent `{}` in config",
                selected.setup.placement.agent
            ),
        ));
    }

    if selected.setup.profile.active.is_none() {
        issues.push(Issue::info(
            "config.no_active_profile",
            "no active profile; run `skm use-profile <profile>`",
        ));
    }

    issues
}

pub fn check_links(
    store: &StorePaths,
    selected: &SelectedSetup,
    active_profile: &str,
) -> Result<Vec<Issue>, SkmError> {
    let mut issues = Vec::new();

    let profile = match load_profile(store, active_profile) {
        Ok(profile) => profile,
        Err(_) => return Ok(issues),
    };

    let disabled = read_disabled_ids(store)?;
    let placements = match resolve(&profile, store, &disabled) {
        Ok(placements) => placements,
        Err(_) => return Ok(issues),
    };

    let desired_names: HashSet<&str> = placements.iter().map(|p| p.name.as_str()).collect();
    let store_root = store.canonical_root();
    let (_agent, target) = target_dir_for_setup(selected)?;
    if !target.is_dir() {
        return Ok(issues);
    }

    for placement in &placements {
        let link_path = target.join(&placement.name);
        if is_foreign_occupant(&link_path, &store_root) {
            issues.push(
                Issue::info(
                    "link.conflict",
                    format!(
                        "skill `{name}` in profile is conflicted by a non-skm entry",
                        name = placement.name
                    ),
                )
                .with_skill(&placement.store_id)
                .with_profile(active_profile)
                .with_path(display_path(&link_path)),
            );
        }
    }

    walk_store_owned_symlinks(&target, &store_root, |path, rel| {
        let Some(resolved) = resolve_link_target(path) else {
            issues.push(
                Issue::warn(
                    "link.broken",
                    format!("broken symlink `{rel}` in agent skills directory"),
                )
                .with_path(display_path(path)),
            );
            return Ok(());
        };

        if !is_path_inside(&store_root, &resolved) {
            issues.push(
                Issue::warn(
                    "link.stale",
                    format!("symlink `{rel}` points outside the skill store"),
                )
                .with_path(display_path(path)),
            );
            return Ok(());
        }

        if !desired_names.contains(rel.as_str()) {
            issues.push(
                Issue::info(
                    "link.extra",
                    format!("symlink `{rel}` is not in the active profile"),
                )
                .with_path(display_path(path)),
            );
        }

        Ok(())
    })?;

    Ok(issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::init_store_layout;
    use tempfile::TempDir;

    #[test]
    fn check_store_missing() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        let issues = check_store(&store);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "store.missing");
    }

    #[test]
    fn check_profile_missing_ref() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let profiles_dir = store.profiles_dir();
        fs::write(
            profiles_dir.join("work.toml"),
            "[[skill]]\nid = \"missing\"\n",
        )
        .unwrap();

        let issues = check_profiles(&store).unwrap();
        assert!(issues.iter().any(|i| i.code == "profile.missing_ref"));
    }

    #[test]
    fn check_meta_accepts_bundle_level_meta() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let eng = store.root().join("engineering/tdd");
        fs::create_dir_all(&eng).unwrap();
        fs::write(eng.join("SKILL.md"), "# tdd").unwrap();
        fs::write(store.meta_file("engineering"), "version = 1\n").unwrap();

        let issues = check_meta(&store).unwrap();
        assert!(!issues.iter().any(|i| i.code == "meta.orphan"));
        assert!(!issues.iter().any(|i| i.code == "meta.missing"));
    }

    #[test]
    fn check_skills_ignores_internal_skill_subdirs() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let skill = store.root().join("engineering/tdd");
        fs::create_dir_all(skill.join("agents")).unwrap();
        fs::write(skill.join("SKILL.md"), "# tdd").unwrap();
        fs::write(skill.join("agents/prompt.md"), "prompt").unwrap();

        let issues = check_skills_on_disk(&store).unwrap();
        assert!(!issues.iter().any(|i| i.code == "skill.missing_skill_md"));
    }

    #[test]
    fn check_skills_flags_incomplete_top_level_dir() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let broken = store.root().join("broken");
        fs::create_dir_all(&broken).unwrap();
        fs::write(broken.join("README.md"), "oops").unwrap();

        let issues = check_skills_on_disk(&store).unwrap();
        assert!(issues.iter().any(|i| i.code == "skill.missing_skill_md"));
    }
}
