use std::env;

use crate::error::SkmError;
use crate::progress;
use crate::setup::{select_setup, set_active_profile};
use crate::store::profiles::load_profile;
use crate::store::StorePaths;
use crate::sync::{reconcile_for_profile, ReconcileOptions};
use crate::util::validate_profile_name;

pub fn run_use_profile(
    store: &StorePaths,
    profile: &str,
    force_user: bool,
    options: ReconcileOptions,
) -> Result<(), SkmError> {
    validate_profile_name(profile)?;
    let cwd = env::current_dir()?;
    let mut setup = select_setup(&cwd, force_user)?;

    let loaded =
        load_profile(store, profile).map_err(|e| e.op(format!("loading profile `{profile}`")))?;
    if loaded.skill.is_empty() {
        return Err(SkmError::EmptyProfile);
    }

    if options.dry_run {
        progress::step(format!("(dry-run) activating profile `{profile}`"));
    } else {
        progress::step(format!("activating profile `{profile}`"));
    }
    reconcile_for_profile(store, &setup, profile, options).map_err(|e| e.op("syncing skills"))?;

    if options.dry_run {
        return Ok(());
    }

    set_active_profile(&mut setup, profile).map_err(|e| e.op("setting active profile"))?;
    Ok(())
}
