use std::env;
use std::io::{self, IsTerminal};

use dialoguer::Confirm;

use crate::db::rebuild_from_store;
use crate::error::SkmError;
use crate::progress;
use crate::setup::clear_active_profile_if_empty;
use crate::store::pool::{plan_skill_removal, remove_skill};
use crate::store::profiles::{profiles_referencing_skills, remove_skills_from_profiles};
use crate::store::skills::{interactive_skills_setup, list_enabled_pool_ids, read_disabled_ids};
use crate::store::StorePaths;
use crate::util::validate_store_skill_id;

pub fn run_setup(store: &StorePaths) -> Result<(), SkmError> {
    interactive_skills_setup(store).map_err(|e| e.op("selecting skills interactively"))?;
    rebuild_from_store(store).map_err(|e| e.op("refreshing skill index"))?;

    let enabled = list_enabled_pool_ids(store)?;
    let disabled = read_disabled_ids(store)?;
    progress::step(format!(
        "updated skill library ({} enabled, {} disabled)",
        enabled.len(),
        disabled.len()
    ));
    Ok(())
}

pub fn run_ls(store: &StorePaths, json: bool) -> Result<(), SkmError> {
    super::ls::run(store, super::ls::LsFilter::Skills, json)
}

pub fn run_rm(store: &StorePaths, id: &str, force: bool, dry_run: bool) -> Result<(), SkmError> {
    validate_store_skill_id(id)?;
    let plan = plan_skill_removal(store, id)
        .map_err(|e| e.op(format!("planning removal of skill `{id}`")))?;

    let cwd = env::current_dir()?;

    if dry_run {
        let refs = profiles_referencing_skills(store, &plan.skill_ids)
            .map_err(|e| e.op(format!("listing profiles referencing skill `{id}`")))?;
        progress::step(format!(
            "(dry-run) would remove skill ids: {}",
            plan.skill_ids.join(", ")
        ));
        progress::step(format!(
            "(dry-run) would delete store path: {}",
            progress::display_path(&plan.remove_root)
        ));
        if refs.is_empty() {
            progress::step("(dry-run) no profiles reference this skill");
        } else {
            progress::step(format!(
                "(dry-run) would update profiles: {}",
                refs.join(", ")
            ));
        }
        return Ok(());
    }

    if !force {
        if !io::stdin().is_terminal() {
            return Err(SkmError::RefuseNonInteractiveRm);
        }

        let mut message = format!(
            "Remove skill `{id}` from the library? This cannot be undone.\n\
             Symlinks will be removed on the next `skm sync`."
        );
        let refs = profiles_referencing_skills(store, &plan.skill_ids)
            .map_err(|e| e.op(format!("listing profiles referencing skill `{id}`")))?;
        if !refs.is_empty() {
            message.push_str(&format!(
                "\nWill be removed from profiles: {}",
                refs.join(", ")
            ));
        }

        match Confirm::new()
            .with_prompt(message)
            .default(false)
            .interact_opt()
            .map_err(|_| SkmError::SelectionCancelled)?
        {
            Some(true) => {}
            _ => return Ok(()),
        }
    }

    let updated_profiles = remove_skills_from_profiles(store, &plan.skill_ids)
        .map_err(|e| e.op(format!("updating profiles for skill `{id}`")))?;
    remove_skill(store, &plan).map_err(|e| e.op(format!("removing skill `{id}`")))?;

    for profile_name in &updated_profiles {
        clear_active_profile_if_empty(&cwd, store, profile_name)
            .map_err(|e| e.op(format!("clearing active profile `{profile_name}`")))?;
    }

    if updated_profiles.is_empty() {
        progress::step(format!("removed skill {id}"));
    } else {
        progress::step(format!(
            "removed skill {id} (updated profiles: {})",
            updated_profiles.join(", ")
        ));
    }
    Ok(())
}
