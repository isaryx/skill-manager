use std::env;
use std::io::{self, IsTerminal};

use crate::error::SkmError;
use crate::progress;
use crate::setup::select_setup;
use crate::store::profiles::{
    create_profile, ensure_profile_not_active, interactive_setup, load_profile, remove_profile,
    set_profile_skills,
};
use crate::store::skills::read_disabled_ids;
use crate::store::StorePaths;
use crate::util::validate_profile_name;

pub fn run_profile_setup(store: &StorePaths, name: &str) -> Result<(), SkmError> {
    validate_profile_name(name)?;
    if !io::stdin().is_terminal() {
        return Err(SkmError::NotATty);
    }

    let existed = store.profile_file(name).is_file();
    let existing: Vec<String> = if existed {
        load_profile(store, name)
            .map_err(|e| e.op(format!("loading profile `{name}`")))?
            .skill
            .into_iter()
            .map(|s| s.id)
            .collect()
    } else {
        Vec::new()
    };

    let skill_ids = interactive_setup(store, name, &existing)
        .map_err(|e| e.op(format!("setting up profile `{name}`")))?;

    if !existed {
        create_profile(store, name, &skill_ids)
            .map_err(|e| e.op(format!("creating profile `{name}`")))?;
    } else {
        set_profile_skills(store, name, &skill_ids)
            .map_err(|e| e.op(format!("writing profile `{name}`")))?;
    }

    let verb = if existed { "updated" } else { "created" };
    progress::step(format!(
        "{verb} profile `{name}` with {} skill(s)",
        skill_ids.len()
    ));
    Ok(())
}

pub fn run_profile_ls(store: &StorePaths) -> Result<(), SkmError> {
    crate::cli::ls::run(store, crate::cli::ls::LsFilter::Profiles, false)
}

pub fn run_profile_show(store: &StorePaths, name: &str, force_user: bool) -> Result<(), SkmError> {
    validate_profile_name(name)?;
    let profile =
        load_profile(store, name).map_err(|e| e.op(format!("loading profile `{name}`")))?;

    let cwd = env::current_dir()?;
    if let Ok(setup) = select_setup(&cwd, force_user) {
        if setup.setup.profile.active.as_deref() == Some(name) {
            eprintln!("(active)");
        }
    }

    let disabled = read_disabled_ids(store)?;

    for entry in &profile.skill {
        if disabled.contains(&entry.id) {
            println!("{} (disabled)", entry.id);
        } else {
            println!("{}", entry.id);
        }
    }
    Ok(())
}

pub fn run_profile_rm(store: &StorePaths, name: &str) -> Result<(), SkmError> {
    validate_profile_name(name)?;
    let cwd = env::current_dir()?;
    ensure_profile_not_active(&cwd, name)
        .map_err(|e| e.op(format!("checking active profile for `{name}`")))?;
    remove_profile(store, name).map_err(|e| e.op(format!("removing profile `{name}`")))?;
    progress::step(format!("removed profile `{name}`"));
    Ok(())
}
