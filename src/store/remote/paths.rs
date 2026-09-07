use std::path::PathBuf;

use crate::store::StorePaths;

pub const CHECKOUT_REL_PREFIX: &str = "remotes/";

pub fn remotes_dir(store: &StorePaths) -> PathBuf {
    store.skm_dir().join("remotes")
}

pub fn repos_dir(store: &StorePaths) -> PathBuf {
    store.skm_dir().join("repos")
}

pub fn repo_registry_file(store: &StorePaths, name: &str) -> PathBuf {
    repos_dir(store).join(format!("{name}.toml"))
}

pub fn checkout_path(store: &StorePaths, name: &str) -> PathBuf {
    remotes_dir(store).join(name)
}

pub fn checkout_relative(name: &str) -> String {
    format!("{CHECKOUT_REL_PREFIX}{name}")
}
