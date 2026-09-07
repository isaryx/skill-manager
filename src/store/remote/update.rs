use chrono::Utc;

use crate::config::SkillMeta;
use crate::db::rebuild_from_store;
use crate::error::SkmError;
use crate::progress;
use crate::store::remote::discover::{find_skills_root, list_repo_skills};
use crate::store::remote::git::update_checkout;
use crate::store::remote::link::refresh_library_symlinks;
use crate::store::remote::meta::{record_remote_skill_sync, RemoteSyncRecord};
use crate::store::remote::paths::checkout_path;
use crate::store::remote::registry::{list_repos, read_repo, write_repo};
use crate::store::{write_meta, StorePaths};
use crate::util::hash_directory;

#[derive(Debug, Clone, Default)]
pub struct PullOptions {
    pub dry_run: bool,
    pub repo_filter: Option<String>,
}

pub fn pull_remotes(store: &StorePaths, options: PullOptions) -> Result<(), SkmError> {
    store.ensure_initialized()?;

    let repos: Vec<_> = match &options.repo_filter {
        Some(name) => vec![read_repo(store, name)?],
        None => list_repos(store)?,
    };

    if repos.is_empty() {
        return Ok(());
    }

    let mut any_pulled = false;
    for reg in repos {
        if options.dry_run {
            progress::step(format!(
                "(dry-run) would pull remote `{}` from {}",
                reg.name,
                reg.url
            ));
            continue;
        }

        let checkout = checkout_path(store, &reg.name);
        if !checkout.is_dir() {
            eprintln!(
                "warning: checkout missing for `{}`: {}",
                reg.name,
                checkout.display()
            );
            continue;
        }

        match update_checkout(&checkout, reg.pin.as_deref()) {
            Ok(commit) => {
                if let Err(err) = refresh_repo_after_pull(store, &reg.name, &commit) {
                    eprintln!("warning: refresh failed for `{}`: {}", reg.name, err.leaf());
                }
                any_pulled = true;
            }
            Err(err) => {
                eprintln!("warning: pull failed for `{}`: {}", reg.name, err.leaf());
                let mut failed = reg;
                failed.last_pull_error = Some(err.leaf().to_string());
                write_repo(store, &failed)?;
            }
        }
    }

    if any_pulled && !options.dry_run {
        rebuild_from_store(store)?;
    }

    Ok(())
}

pub fn refresh_repo_after_pull(
    store: &StorePaths,
    name: &str,
    commit: &str,
) -> Result<(), SkmError> {
    let reg = read_repo(store, name)?;
    let checkout = checkout_path(store, &reg.name);

    let skills_root = match find_skills_root(&checkout)? {
        Some(root) => root,
        None => {
            refresh_library_symlinks(store, &reg.name, &[])?;
            let mut updated = reg;
            updated.commit = commit.to_string();
            updated.updated_at = Utc::now().to_rfc3339();
            updated.last_pull_error = None;
            write_repo(store, &updated)?;
            return Ok(());
        }
    };

    let skills = list_repo_skills(&checkout, &skills_root, &reg.name)?;
    refresh_library_symlinks(store, &reg.name, &skills)?;

    let checkout_abs = checkout
        .canonicalize()
        .unwrap_or_else(|_| checkout.clone());
    let meta = SkillMeta {
        source_type: "remote".to_string(),
        path: checkout_abs.to_string_lossy().into_owned(),
        hash: hash_directory(&checkout)?,
        imported_at: reg.cloned_at.clone(),
        transfer: "clone".to_string(),
        repo_name: Some(reg.name.clone()),
        remote_url: Some(reg.url.clone()),
        commit: Some(commit.to_string()),
        synced_at: None,
    };
    write_meta(store, &reg.name, &toml::to_string_pretty(&meta)?)?;

    let mut updated = reg;
    updated.commit = commit.to_string();
    updated.skills_root = skills_root.segment;
    updated.updated_at = Utc::now().to_rfc3339();
    updated.last_pull_error = None;
    write_repo(store, &updated)?;

    record_remote_skill_sync(store, &updated, &skills, &checkout, RemoteSyncRecord::Full)?;

    Ok(())
}

pub fn skill_count_for_repo(store: &StorePaths, name: &str) -> Result<usize, SkmError> {
    let checkout = checkout_path(store, name);
    if !checkout.is_dir() {
        return Ok(0);
    }
    match find_skills_root(&checkout)? {
        None => Ok(0),
        Some(root) => list_repo_skills(&checkout, &root, name).map(|skills| skills.len()),
    }
}
