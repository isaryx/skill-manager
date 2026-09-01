use std::fs;
use std::io::{self, IsTerminal};
use std::path::Path;

use crate::config::{ProfileFile, ProfileSkillEntry};
use crate::error::SkmError;
use crate::store::skills::read_disabled_ids;
use crate::store::{list_pool_ids, StorePaths};
use crate::tui::{MultiSelect, MultiSelectItem};
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
        extends: Vec::new(),
        skill: skill_entries(skill_ids),
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

    // Read-modify-write: rebuilding the file from scratch here would drop `extends`.
    let mut profile = read_profile(&path)?;
    profile.skill = skill_entries(skill_ids);
    write_profile(store, name, &profile)
}

/// Replace the profiles `name` extends, leaving its own skill list untouched.
pub fn set_profile_extends(
    store: &StorePaths,
    name: &str,
    extends: &[String],
) -> Result<(), SkmError> {
    store.ensure_initialized()?;
    validate_profile_name(name)?;
    let path = store.profile_file(name);
    if !path.is_file() {
        return Err(SkmError::ProfileNotFound(name.to_string()));
    }
    validate_extends(name, extends)?;

    let mut profile = read_profile(&path)?;
    profile.extends = extends.to_vec();
    write_profile(store, name, &profile)
}

/// Replace the profiles `name` extends, creating `name` if it does not exist yet.
///
/// One file write either way. `ensure_profile` followed by [`set_profile_extends`] would write an
/// empty profile, read it straight back, and write it again — and a failure between the two would
/// leave the empty profile behind, which is exactly what a caller that bails out does not want.
pub fn upsert_profile_extends(
    store: &StorePaths,
    name: &str,
    extends: &[String],
) -> Result<(), SkmError> {
    store.ensure_initialized()?;
    validate_profile_name(name)?;
    validate_extends(name, extends)?;

    let path = store.profile_file(name);
    let mut profile = if path.is_file() {
        read_profile(&path)?
    } else {
        ProfileFile::default()
    };
    profile.extends = extends.to_vec();
    write_profile(store, name, &profile)
}

fn skill_entries(skill_ids: &[String]) -> Vec<ProfileSkillEntry> {
    skill_ids
        .iter()
        .map(|id| ProfileSkillEntry { id: id.clone() })
        .collect()
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

    for path in [project_setup_path(cwd), user_setup_path()?] {
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
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    validate_extends(name, &profile.extends)?;
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

/// Pick the skills for profile `name` in a full-screen list. Returns the chosen store IDs.
///
/// The pool is the enabled library plus any disabled skills the profile already references, so
/// editing a profile never silently drops a skill that happens to be hidden right now.
pub fn interactive_setup(
    store: &StorePaths,
    name: &str,
    selected: &[String],
) -> Result<Vec<String>, SkmError> {
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
    let items = pool.iter().map(|id| {
        let item = MultiSelectItem::new(id).selected(selected_set.contains(id.as_str()));
        if disabled.contains(id) {
            item.note("disabled")
        } else {
            item
        }
    });

    MultiSelect::new(format!("Skills for profile `{name}`"))
        .items(items)
        .interact()
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

/// Names must be valid profiles, distinct, and never the profile itself. Whether the resulting
/// graph is acyclic and within the depth limit is checked by `extends::flatten_profile`.
fn validate_extends(name: &str, extends: &[String]) -> Result<(), SkmError> {
    let mut seen = std::collections::HashSet::new();
    for target in extends {
        validate_profile_name(target)?;
        if target == name {
            return Err(SkmError::SelfExtend(name.to_string()));
        }
        if !seen.insert(target.clone()) {
            return Err(SkmError::DuplicateExtend(target.clone()));
        }
    }
    Ok(())
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
    use std::fs;
    use tempfile::TempDir;

    /// `extends` is a TOML value and `[[skill]]` an array of tables, so the value has to be
    /// emitted first — the `toml` crate refuses values after tables.
    #[test]
    fn profile_with_extends_and_skills_roundtrips() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        create_profile(&store, "work", &["docx".to_string(), "git".to_string()]).unwrap();
        set_profile_extends(&store, "work", &["base".to_string(), "infra".to_string()]).unwrap();

        let body = fs::read_to_string(store.profile_file("work")).unwrap();
        assert!(
            body.find("extends").unwrap() < body.find("[[skill]]").unwrap(),
            "{body}"
        );

        let profile = load_profile(&store, "work").unwrap();
        assert_eq!(profile.extends, vec!["base", "infra"]);
        assert_eq!(profile.skill.len(), 2);
    }

    #[test]
    fn setting_skills_keeps_extends() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        create_profile(&store, "work", &["docx".to_string()]).unwrap();
        set_profile_extends(&store, "work", &["base".to_string()]).unwrap();
        set_profile_skills(&store, "work", &["git".to_string()]).unwrap();

        let profile = load_profile(&store, "work").unwrap();
        assert_eq!(
            profile.extends,
            vec!["base"],
            "extends must survive a skill rewrite"
        );
        assert_eq!(profile.skill[0].id, "git");
    }

    #[test]
    fn setting_extends_keeps_skills() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        create_profile(&store, "work", &["docx".to_string()]).unwrap();
        set_profile_extends(&store, "work", &["base".to_string()]).unwrap();

        let profile = load_profile(&store, "work").unwrap();
        assert_eq!(profile.skill.len(), 1);
        assert_eq!(profile.skill[0].id, "docx");
    }

    #[test]
    fn self_extend_is_rejected() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        create_profile(&store, "work", &[]).unwrap();
        let err = set_profile_extends(&store, "work", &["work".to_string()]).unwrap_err();
        assert!(matches!(err, SkmError::SelfExtend(_)), "{err:?}");
    }

    /// The write path needs its own guard: `set_profile_extends` would otherwise persist a name
    /// that only fails later, when something tries to read it as a path.
    #[test]
    fn set_profile_extends_rejects_names_that_escape_the_profiles_directory() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();
        create_profile(&store, "app", &[]).unwrap();

        for hostile in ["../../etc/passwd", "..", "/etc/passwd", "sub/dir", ".skm"] {
            let err = set_profile_extends(&store, "app", &[hostile.to_string()]).unwrap_err();
            assert!(
                matches!(
                    err,
                    SkmError::InvalidProfileName(_) | SkmError::ReservedName(_)
                ),
                "{hostile:?} was not rejected: {err:?}"
            );
        }

        // Nothing hostile was written.
        assert!(load_profile(&store, "app").unwrap().extends.is_empty());
    }

    /// The create path writes once, so a profile that did not exist comes into being already
    /// carrying its `extends` — there is no intermediate empty profile to leave behind.
    #[test]
    fn upsert_extends_creates_a_missing_profile_and_preserves_an_existing_ones_skills() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        upsert_profile_extends(&store, "fresh", &["base".to_string()]).unwrap();
        let created = load_profile(&store, "fresh").unwrap();
        assert_eq!(created.extends, vec!["base"]);
        assert!(created.skill.is_empty());

        create_profile(&store, "work", &["docx".to_string()]).unwrap();
        upsert_profile_extends(&store, "work", &["base".to_string()]).unwrap();
        let updated = load_profile(&store, "work").unwrap();
        assert_eq!(updated.extends, vec!["base"]);
        assert_eq!(updated.skill[0].id, "docx", "own skills must survive");
    }

    #[test]
    fn upsert_extends_rejects_the_same_names_as_the_strict_setter() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let err = upsert_profile_extends(&store, "app", &["app".to_string()]).unwrap_err();
        assert!(matches!(err, SkmError::SelfExtend(_)), "{err:?}");
        // `ReservedName` rather than `InvalidProfileName`: the leading `.` is checked first.
        let err =
            upsert_profile_extends(&store, "app", &["../../etc/passwd".to_string()]).unwrap_err();
        assert!(
            matches!(
                err,
                SkmError::InvalidProfileName(_) | SkmError::ReservedName(_)
            ),
            "{err:?}"
        );

        // A rejected upsert must not have created the profile as a side effect.
        assert!(!store.profile_file("app").is_file());
    }

    #[test]
    fn duplicate_extends_are_rejected() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        create_profile(&store, "work", &[]).unwrap();
        let err = set_profile_extends(&store, "work", &["base".to_string(), "base".to_string()])
            .unwrap_err();
        assert!(matches!(err, SkmError::DuplicateExtend(_)), "{err:?}");
    }

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

    #[test]
    fn read_profile_rejects_duplicate_skill_ids() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        fs::write(
            store.profile_file("dup"),
            "[[skill]]\nid = \"docx\"\n\n[[skill]]\nid = \"docx\"\n",
        )
        .unwrap();

        let err = load_profile(&store, "dup").unwrap_err();
        assert!(matches!(err, SkmError::DuplicateSkillId(id) if id == "docx"));
    }
}
