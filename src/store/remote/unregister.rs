use std::fs;
use std::io::{self, IsTerminal};

use dialoguer::Confirm;

use crate::db::rebuild_from_store;
use crate::error::SkmError;
use crate::progress;
use crate::store::profiles::{
    load_profile, profiles_referencing_repo_prefix, remove_skills_from_profiles,
};
use crate::store::remote::link::remove_repo_library_links;
use crate::store::remote::meta::remove_remote_skill_meta;
use crate::store::remote::paths::{checkout_path, repo_registry_file};
use crate::store::remote::registry::read_repo;
use crate::store::StorePaths;

pub fn unregister_repo(
    store: &StorePaths,
    name: &str,
    force: bool,
) -> Result<Vec<String>, SkmError> {
    store.ensure_initialized()?;
    read_repo(store, name)?;

    let refs = profiles_referencing_repo_prefix(store, name)?;
    if !refs.is_empty() && !force {
        return Err(SkmError::RepoReferencedByProfiles {
            name: name.to_string(),
            profiles: refs.join(", "),
        });
    }

    if !force {
        if !io::stdin().is_terminal() {
            return Err(SkmError::RefuseNonInteractiveRm);
        }
        let mut message = format!(
            "Remove remote repository `{name}` from the store? This deletes the checkout and library symlinks."
        );
        if !refs.is_empty() {
            message.push_str(&format!(
                "\nProfiles still reference skills under this repo: {}",
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
            _ => return Ok(Vec::new()),
        }
    }

    progress::step(format!("removing remote repository `{name}`"));

    let mut updated_profiles = Vec::new();
    if force && !refs.is_empty() {
        let skill_ids = repo_skill_ids_for_removal(store, name)?;
        updated_profiles = remove_skills_from_profiles(store, &skill_ids)?;
        if !updated_profiles.is_empty() {
            progress::step(format!("updated profiles: {}", updated_profiles.join(", ")));
        }
    }

    let skill_ids = {
        use crate::store::remote::link::collect_repo_library_links;
        collect_repo_library_links(store, name)?
    };
    for id in &skill_ids {
        remove_remote_skill_meta(store, id)?;
    }

    remove_repo_library_links(store, name)?;
    remove_checkout(store, name)?;
    remove_registry(store, name)?;
    remove_bundle_meta(store, name)?;

    rebuild_from_store(store)?;
    Ok(updated_profiles)
}

fn remove_checkout(store: &StorePaths, name: &str) -> Result<(), SkmError> {
    let checkout = checkout_path(store, name);
    if checkout.exists() {
        fs::remove_dir_all(&checkout)?;
    }
    Ok(())
}

fn remove_registry(store: &StorePaths, name: &str) -> Result<(), SkmError> {
    let path = repo_registry_file(store, name);
    if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn remove_bundle_meta(store: &StorePaths, name: &str) -> Result<(), SkmError> {
    let path = store.meta_file(name);
    if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn repo_skill_ids_for_removal(store: &StorePaths, name: &str) -> Result<Vec<String>, SkmError> {
    let prefix = format!("{name}/");
    let mut ids = Vec::new();
    for profile in profiles_referencing_repo_prefix(store, name)? {
        for entry in load_profile(store, &profile)?.skill {
            if (entry.id == name || entry.id.starts_with(&prefix)) && !ids.contains(&entry.id) {
                ids.push(entry.id);
            }
        }
    }
    Ok(ids)
}
