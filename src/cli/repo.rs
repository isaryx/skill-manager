use std::env;

use dialoguer::console::Term;

use crate::error::SkmError;
use crate::progress;
use crate::setup::clear_active_profile_if_empty;
use crate::store::remote::skills_sh::{
    fetch_leaderboard, format_installs, registered_github_owner_repos,
};
use crate::store::remote::{
    clear_repo_pin, list_repos, register_repo, set_repo_pin, show_repo_pin, skill_count_for_repo,
    unregister_repo,
};
use crate::store::StorePaths;
use crate::tui::{MultiSelect, MultiSelectItem};
use crate::util::validate_store_entry_name;

pub fn run_repo_add(
    store: &StorePaths,
    ref_str: &str,
    name: Option<&str>,
    strict: bool,
    pin: Option<&str>,
) -> Result<(), SkmError> {
    if let Some(n) = name {
        validate_store_entry_name(n)?;
    }
    progress::step(format!("registering remote repository `{ref_str}`"));
    register_repo(store, ref_str, name, strict, pin)?;
    Ok(())
}

pub fn run_repo_rm(store: &StorePaths, name: &str, force: bool) -> Result<(), SkmError> {
    validate_store_entry_name(name)?;
    let updated_profiles = unregister_repo(store, name, force)?;
    let cwd = env::current_dir()?;
    for profile_name in &updated_profiles {
        clear_active_profile_if_empty(&cwd, store, profile_name)?;
    }
    Ok(())
}

pub fn run_repo_pin(
    store: &StorePaths,
    name: &str,
    pin: Option<&str>,
    clear: bool,
) -> Result<(), SkmError> {
    validate_store_entry_name(name)?;
    if clear {
        if pin.is_some() {
            return Err(SkmError::Usage(
                "pass either a ref or --clear, not both".into(),
            ));
        }
        clear_repo_pin(store, name)?;
        progress::step(format!("cleared pin for remote `{name}` (tracks default branch)"));
        return Ok(());
    }
    match pin {
        Some(pin) => {
            set_repo_pin(store, name, pin)?;
            progress::step(format!("pinned remote `{name}` to `{pin}`"));
        }
        None => show_repo_pin(store, name)?,
    }
    Ok(())
}

pub fn run_repo_browse(store: &StorePaths, strict: bool) -> Result<(), SkmError> {
    let term = Term::stderr();
    if !term.is_term() {
        return Err(SkmError::NotATty);
    }
    store.ensure_initialized()?;

    progress::step("fetching skills.sh leaderboard");
    let leaderboard = fetch_leaderboard()?;

    let registered = registered_github_owner_repos(&list_repos(store)?);
    let items = leaderboard.iter().map(|entry| {
        let key = entry.owner_repo();
        let hint = format!(
            "{} skill{} · {} installs",
            entry.skill_count,
            if entry.skill_count == 1 { "" } else { "s" },
            format_installs(entry.installs)
        );
        if registered.contains(&key) {
            MultiSelectItem::new(key)
                .note(format!("{hint}, registered"))
                .selectable(false)
        } else {
            MultiSelectItem::new(key).hint(hint)
        }
    });

    let selected = MultiSelect::new("skills.sh leaderboard — select repositories to add")
        .items(items)
        .interact()?;

    for ref_str in selected {
        run_repo_add(store, &ref_str, None, strict, None)?;
    }
    Ok(())
}

pub fn run_repo_ls(store: &StorePaths, json: bool) -> Result<(), SkmError> {
    let repos = list_repos(store)?;
    if json {
        let mut payload = Vec::new();
        for reg in &repos {
            payload.push(RepoJson {
                name: reg.name.clone(),
                url: reg.url.clone(),
                commit: reg.commit.clone(),
                skills_root: reg.skills_root.clone(),
                skill_count: skill_count_for_repo(store, &reg.name)?,
                updated_at: reg.updated_at.clone(),
                pin: reg.pin.clone(),
            });
        }
        let body = serde_json::json!({ "repos": payload });
        println!(
            "{}",
            serde_json::to_string_pretty(&body)
                .map_err(|e| SkmError::Usage(format!("failed to encode JSON: {e}")))?
        );
        return Ok(());
    }

    if repos.is_empty() {
        return Ok(());
    }

    for reg in &repos {
        let short_commit = reg.commit.chars().take(7).collect::<String>();
        let count = skill_count_for_repo(store, &reg.name)?;
        let pin = reg
            .pin
            .as_deref()
            .map(|p| format!(" pin={p}"))
            .unwrap_or_default();
        println!(
            "{}  {}  {}  {} skill{}{}",
            reg.name,
            reg.url,
            short_commit,
            count,
            if count == 1 { "" } else { "s" },
            pin
        );
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct RepoJson {
    name: String,
    url: String,
    commit: String,
    skills_root: String,
    skill_count: usize,
    updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pin: Option<String>,
}
