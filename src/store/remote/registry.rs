use std::fs;
use std::path::Path;

use crate::config::RepoRegistration;
use crate::error::SkmError;
use crate::store::remote::paths::{repo_registry_file, repos_dir};
use crate::store::remote::url::normalize_url;
use crate::store::StorePaths;

pub fn list_repos(store: &StorePaths) -> Result<Vec<RepoRegistration>, SkmError> {
    let dir = repos_dir(store);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut repos = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        repos.push(read_repo_file(&path)?);
    }
    repos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(repos)
}

pub fn read_repo(store: &StorePaths, name: &str) -> Result<RepoRegistration, SkmError> {
    let path = repo_registry_file(store, name);
    if !path.is_file() {
        return Err(SkmError::RepoNotFound(name.to_string()));
    }
    read_repo_file(&path)
}

pub fn write_repo(store: &StorePaths, reg: &RepoRegistration) -> Result<(), SkmError> {
    fs::create_dir_all(repos_dir(store))?;
    let content = toml::to_string_pretty(reg)?;
    fs::write(repo_registry_file(store, &reg.name), content)?;
    Ok(())
}

pub fn find_by_url(store: &StorePaths, url: &str) -> Result<Option<RepoRegistration>, SkmError> {
    let needle = normalize_url(url);
    for reg in list_repos(store)? {
        if normalize_url(&reg.url) == needle {
            return Ok(Some(reg));
        }
    }
    Ok(None)
}

pub fn ensure_not_registered(store: &StorePaths, name: &str, url: &str) -> Result<(), SkmError> {
    let path = repo_registry_file(store, name);
    if path.is_file() {
        return Err(SkmError::RepoAlreadyRegistered(name.to_string()));
    }
    if let Some(existing) = find_by_url(store, url)? {
        return Err(SkmError::RepoUrlAlreadyRegistered {
            url: url.to_string(),
            existing_name: existing.name,
        });
    }
    Ok(())
}

fn read_repo_file(path: &Path) -> Result<RepoRegistration, SkmError> {
    let content = fs::read_to_string(path)?;
    let reg: RepoRegistration = toml::from_str(&content).map_err(|e| SkmError::InvalidStore {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    Ok(reg)
}
