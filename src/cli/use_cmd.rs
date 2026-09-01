use std::env;
use std::io::{self, IsTerminal};

use dialoguer::Select;

use crate::error::SkmError;
use crate::progress;
use crate::setup::{select_setup, set_active_profile};
use crate::store::extends::flatten_skill_ids;
use crate::store::profiles::list_profiles;
use crate::store::StorePaths;
use crate::sync::{reconcile_for_profile, ReconcileOptions};
use crate::tui::sanitize;
use crate::util::validate_profile_name;

pub fn run_use_profile(
    store: &StorePaths,
    profile: Option<&str>,
    force_user: bool,
    options: ReconcileOptions,
) -> Result<(), SkmError> {
    if let Some(profile) = profile {
        validate_profile_name(profile)?;
    }

    let cwd = env::current_dir()?;
    let mut setup = select_setup(&cwd, force_user)?;
    let profile = match profile {
        Some(profile) => profile.to_string(),
        None => interactive_select_profile(store, setup.setup.profile.active.as_deref())?,
    };
    validate_profile_name(&profile)?;

    // Flattened, so a profile that only extends others is not "empty".
    let skills = flatten_skill_ids(store, &profile)
        .map_err(|e| e.op(format!("loading profile `{profile}`")))?;
    if skills.is_empty() {
        return Err(SkmError::EmptyProfile);
    }

    if options.dry_run {
        progress::step(format!("(dry-run) activating profile `{profile}`"));
    } else {
        progress::step(format!("activating profile `{profile}`"));
    }
    reconcile_for_profile(store, &setup, &profile, options).map_err(|e| e.op("syncing skills"))?;

    if options.dry_run {
        return Ok(());
    }

    set_active_profile(&mut setup, &profile).map_err(|e| e.op("setting active profile"))?;
    Ok(())
}

fn interactive_select_profile(
    store: &StorePaths,
    active_profile: Option<&str>,
) -> Result<String, SkmError> {
    let profiles = list_profiles(store)?;
    let (labels, default) = profile_menu(&profiles, active_profile)?;

    if !io::stdin().is_terminal() {
        return Err(SkmError::Usage(
            "profile name is required when stdin is not a TTY; run `skm use-profile <profile>`"
                .to_string(),
        ));
    }

    let selection = Select::new()
        .with_prompt("Profile")
        .items(&labels)
        .default(default)
        .interact_opt()
        .map_err(|_| SkmError::SelectionCancelled)?
        .ok_or(SkmError::SelectionCancelled)?;

    profiles
        .get(selection)
        .cloned()
        .ok_or(SkmError::SelectionCancelled)
}

fn profile_menu(
    profiles: &[String],
    active_profile: Option<&str>,
) -> Result<(Vec<String>, usize), SkmError> {
    if profiles.is_empty() {
        return Err(SkmError::NoProfiles);
    }

    let default = active_profile
        .and_then(|active| profiles.iter().position(|profile| profile == active))
        .unwrap_or(0);
    let labels = profiles
        .iter()
        .map(|profile| {
            let active = active_profile == Some(profile.as_str());
            let label = sanitize(profile);
            if active {
                format!("{label} (active)")
            } else {
                label
            }
        })
        .collect();

    Ok((labels, default))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_menu_marks_and_defaults_to_the_active_profile() {
        let profiles = vec!["personal".to_string(), "work".to_string()];
        let (labels, default) = profile_menu(&profiles, Some("work")).unwrap();

        assert_eq!(labels, vec!["personal", "work (active)"]);
        assert_eq!(default, 1);
    }

    #[test]
    fn profile_menu_defaults_to_the_first_profile_without_an_active_match() {
        let profiles = vec!["personal".to_string(), "work".to_string()];
        let (labels, default) = profile_menu(&profiles, Some("missing")).unwrap();

        assert_eq!(labels, profiles);
        assert_eq!(default, 0);
    }

    #[test]
    fn profile_menu_rejects_an_empty_list() {
        assert!(matches!(profile_menu(&[], None), Err(SkmError::NoProfiles)));
    }
}
