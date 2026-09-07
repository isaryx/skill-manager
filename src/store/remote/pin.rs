use chrono::Utc;

use crate::db::rebuild_from_store;
use crate::error::SkmError;
use crate::store::remote::git::{checkout_default_branch, checkout_ref, current_commit};
use crate::store::remote::paths::checkout_path;
use crate::store::remote::registry::{read_repo, write_repo};
use crate::store::remote::update::refresh_repo_after_pull;
use crate::store::StorePaths;

pub fn set_repo_pin(store: &StorePaths, name: &str, pin: &str) -> Result<(), SkmError> {
    store.ensure_initialized()?;
    let mut reg = read_repo(store, name)?;
    let checkout = checkout_path(store, &name);
    if !checkout.is_dir() {
        return Err(SkmError::Usage(format!(
            "checkout missing for remote `{name}`; run `skm doctor`"
        )));
    }

    reg.pin = Some(pin.to_string());
    checkout_ref(&checkout, pin)?;
    let commit = current_commit(&checkout)?;
    reg.commit = commit.clone();
    reg.updated_at = Utc::now().to_rfc3339();
    reg.last_pull_error = None;
    write_repo(store, &reg)?;
    refresh_repo_after_pull(store, name, &commit)?;
    rebuild_from_store(store)?;
    Ok(())
}

pub fn clear_repo_pin(store: &StorePaths, name: &str) -> Result<(), SkmError> {
    store.ensure_initialized()?;
    let mut reg = read_repo(store, name)?;
    let checkout = checkout_path(store, &name);
    if !checkout.is_dir() {
        return Err(SkmError::Usage(format!(
            "checkout missing for remote `{name}`; run `skm doctor`"
        )));
    }

    checkout_default_branch(&checkout)?;
    reg.pin = None;
    let commit = current_commit(&checkout)?;
    reg.commit = commit.clone();
    reg.updated_at = Utc::now().to_rfc3339();
    reg.last_pull_error = None;
    write_repo(store, &reg)?;
    refresh_repo_after_pull(store, name, &commit)?;
    rebuild_from_store(store)?;
    Ok(())
}

pub fn show_repo_pin(store: &StorePaths, name: &str) -> Result<(), SkmError> {
    let reg = read_repo(store, name)?;
    match &reg.pin {
        Some(pin) => println!("{name} pinned to {pin}"),
        None => println!("{name} tracks default branch"),
    }
    Ok(())
}
