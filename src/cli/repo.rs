use crate::error::SkmError;
use crate::progress;
use crate::store::remote::{list_repos, register_repo, skill_count_for_repo};
use crate::store::StorePaths;
use crate::util::validate_store_entry_name;

pub fn run_repo_add(
    store: &StorePaths,
    ref_str: &str,
    name: Option<&str>,
    strict: bool,
) -> Result<(), SkmError> {
    if let Some(n) = name {
        validate_store_entry_name(n)?;
    }
    progress::step(format!("registering remote repository `{ref_str}`"));
    register_repo(store, ref_str, name, strict)?;
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
        println!(
            "{}  {}  {}  {} skill{}",
            reg.name,
            reg.url,
            short_commit,
            count,
            if count == 1 { "" } else { "s" }
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
}
