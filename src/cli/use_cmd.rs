use std::env;
use std::io::{self, IsTerminal};

use crate::error::SkmError;
use crate::progress;
use crate::setup::{select_command_setup, set_active_profiles};
use crate::store::extends::load_merged_flattened_profile;
use crate::store::profiles::list_profiles;
use crate::store::StorePaths;
use crate::sync::{reconcile_for_profiles, ReconcileOptions};
use crate::tui::{MultiSelect, MultiSelectItem};
use crate::util::validate_profile_name;

pub fn run_use_profiles(store: &StorePaths, force_user: bool) -> Result<(), SkmError> {
    if !io::stdin().is_terminal() {
        return Err(SkmError::NotATty);
    }

    let cwd = env::current_dir()?;
    let setup = select_command_setup(&cwd, force_user)?;
    let profiles = list_profiles(store)?;
    let chosen = interactive_select_profiles(&profiles, &setup.setup.profile.active)?;
    if !chosen.is_empty() {
        ensure_nonempty_selection(store, &chosen)?;
    }
    apply_active_profiles(store, setup, &chosen, ReconcileOptions { dry_run: false })
}

pub fn run_add_profile(
    store: &StorePaths,
    profile: &str,
    force_user: bool,
    options: ReconcileOptions,
) -> Result<(), SkmError> {
    validate_profile_name(profile)?;

    let cwd = env::current_dir()?;
    let setup = select_command_setup(&cwd, force_user)?;
    if setup.setup.profile.is_active(profile) {
        eprintln!("active profiles unchanged: {}", setup.setup.profile.active.join(", "));
        return Ok(());
    }
    let mut active = setup.setup.profile.active.clone();
    active.push(profile.to_string());
    ensure_nonempty_selection(store, &active)?;
    apply_active_profiles(store, setup, &active, options)
}

pub fn run_remove_profile(
    store: &StorePaths,
    profile: &str,
    force_user: bool,
    options: ReconcileOptions,
) -> Result<(), SkmError> {
    validate_profile_name(profile)?;

    let cwd = env::current_dir()?;
    let setup = select_command_setup(&cwd, force_user)?;
    if !setup.setup.profile.is_active(profile) {
        return Err(SkmError::ProfileNotActive(profile.to_string()));
    }

    let active: Vec<String> = setup
        .setup
        .profile
        .active
        .iter()
        .filter(|name| name.as_str() != profile)
        .cloned()
        .collect();
    apply_active_profiles(store, setup, &active, options)
}

fn apply_active_profiles(
    store: &StorePaths,
    mut setup: crate::setup::SelectedSetup,
    profiles: &[String],
    options: ReconcileOptions,
) -> Result<(), SkmError> {
    if options.dry_run {
        if profiles.is_empty() {
            progress::step("(dry-run) clearing active profiles");
        } else {
            progress::step(format!(
                "(dry-run) activating profile{} `{}`",
                if profiles.len() == 1 { "" } else { "s" },
                profiles.join("`, `")
            ));
        }
    } else if profiles.is_empty() {
        progress::step("clearing active profiles");
    } else {
        progress::step(format!(
            "activating profile{} `{}`",
            if profiles.len() == 1 { "" } else { "s" },
            profiles.join("`, `")
        ));
    }

    reconcile_for_profiles(store, &setup, profiles, options).map_err(|e| e.op("syncing skills"))?;

    if options.dry_run {
        return Ok(());
    }

    set_active_profiles(&mut setup, profiles).map_err(|e| e.op("setting active profiles"))?;
    Ok(())
}

fn ensure_nonempty_selection(store: &StorePaths, profiles: &[String]) -> Result<(), SkmError> {
    if profiles.is_empty() {
        return Ok(());
    }
    let merged = load_merged_flattened_profile(store, profiles)
        .map_err(|e| e.op("loading selected profiles"))?;
    if merged.skill.is_empty() {
        return Err(SkmError::EmptyProfile);
    }
    Ok(())
}

fn interactive_select_profiles(
    profiles: &[String],
    active_profiles: &[String],
) -> Result<Vec<String>, SkmError> {
    if profiles.is_empty() {
        return Err(SkmError::NoProfiles);
    }

    let active_set: std::collections::HashSet<&str> =
        active_profiles.iter().map(String::as_str).collect();
    let items = profiles.iter().map(|profile| {
        MultiSelectItem::new(profile).selected(active_set.contains(profile.as_str()))
    });

    MultiSelect::new("Active profiles")
        .items(items)
        .interact()
        .map_err(|_| SkmError::SelectionCancelled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_selection_requires_profiles() {
        assert!(matches!(
            interactive_select_profiles(&[], &[]),
            Err(SkmError::NoProfiles)
        ));
    }
}
