use std::fs;
use std::path::PathBuf;

use chrono::Utc;

use crate::config::{RepoRegistration, SkillMeta};
use crate::db::rebuild_from_store;
use crate::error::SkmError;
use crate::store::remote::discover::{find_skills_root, list_repo_skills};
use crate::store::remote::git::{clone_repo, current_commit};
use crate::store::remote::link::install_library_symlinks;
use crate::store::remote::paths::{checkout_path, checkout_relative, remotes_dir};
use crate::store::remote::registry::{ensure_not_registered, write_repo};
use crate::store::remote::url::{name_from_url, resolve_url};
use crate::store::{write_meta, StorePaths};
use crate::util::skill_spec::check_skill_specs_for_import;
use crate::util::{hash_directory, validate_store_entry_name};

pub fn register_repo(
    store: &StorePaths,
    ref_str: &str,
    name_override: Option<&str>,
    strict: bool,
) -> Result<Vec<String>, SkmError> {
    store.ensure_initialized()?;

    let url = resolve_url(ref_str)?;
    let name = match name_override {
        Some(n) => n.to_string(),
        None => name_from_url(ref_str)?,
    };
    validate_store_entry_name(&name)?;
    ensure_not_registered(store, &name, &url)?;

    fs::create_dir_all(remotes_dir(store))?;
    let checkout = checkout_path(store, &name);

    if let Err(err) = clone_repo(&url, &checkout) {
        if checkout.exists() {
            let _ = fs::remove_dir_all(&checkout);
        }
        return Err(err);
    }

    let skills_root = match find_skills_root(&checkout)? {
        Some(root) => root,
        None => {
            eprintln!(
                "warning: no skills discovered in repository `{name}`; registry kept for retry"
            );
            write_empty_registration(store, &name, &url, &checkout)?;
            rebuild_from_store(store)?;
            return Ok(Vec::new());
        }
    };

    let skills = list_repo_skills(&checkout, &skills_root, &name)?;
    let skill_paths: Vec<PathBuf> = skills.iter().map(|(_, p)| p.clone()).collect();
    if let Err(err) = check_skill_specs_for_import(&skill_paths, strict, None) {
        let _ = fs::remove_dir_all(&checkout);
        return Err(err);
    }

    let commit = current_commit(&checkout)?;
    let now = Utc::now().to_rfc3339();
    let reg = RepoRegistration {
        version: 1,
        name: name.clone(),
        url: url.clone(),
        checkout: checkout_relative(&name),
        commit: commit.clone(),
        skills_root: skills_root.segment.clone(),
        cloned_at: now.clone(),
        updated_at: now.clone(),
        last_pull_error: None,
    };
    write_repo(store, &reg)?;

    let checkout_abs = checkout
        .canonicalize()
        .unwrap_or_else(|_| checkout.clone());
    let meta = SkillMeta {
        source_type: "remote".to_string(),
        path: checkout_abs.to_string_lossy().into_owned(),
        hash: hash_directory(&checkout)?,
        imported_at: now,
        transfer: "clone".to_string(),
        repo_name: Some(name.clone()),
        remote_url: Some(url),
        commit: Some(commit),
    };
    write_meta(store, &name, &toml::to_string_pretty(&meta)?)?;

    let installed = install_library_symlinks(store, &name, &skills)?;
    rebuild_from_store(store)?;

    if installed.is_empty() {
        eprintln!("warning: no skills linked from repository `{name}`");
    }

    for id in &installed {
        println!("{id}");
    }
    Ok(installed)
}

fn write_empty_registration(
    store: &StorePaths,
    name: &str,
    url: &str,
    checkout: &PathBuf,
) -> Result<(), SkmError> {
    let commit = current_commit(checkout)?;
    let now = Utc::now().to_rfc3339();
    let reg = RepoRegistration {
        version: 1,
        name: name.to_string(),
        url: url.to_string(),
        checkout: checkout_relative(name),
        commit,
        skills_root: String::new(),
        cloned_at: now.clone(),
        updated_at: now,
        last_pull_error: None,
    };
    write_repo(store, &reg)?;
    Ok(())
}
