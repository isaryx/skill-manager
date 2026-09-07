use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::SkmError;
use crate::store::remote::meta::remove_remote_skill_meta;
use crate::store::StorePaths;
use crate::util::is_skill_dir;

/// Install or refresh library symlinks for discovered remote skills.
pub fn install_library_symlinks(
    store: &StorePaths,
    _repo_name: &str,
    skills: &[(String, PathBuf)],
) -> Result<Vec<String>, SkmError> {
    let mut installed = Vec::new();
    for (id, target) in skills {
        if !is_skill_dir(target) {
            continue;
        }
        match install_one_symlink(store, id, target) {
            Ok(()) => installed.push(id.clone()),
            Err(SkmError::LibraryLinkOccupied { .. }) => {
                eprintln!(
                    "warning: skipping `{}`: destination occupied by non-skm entry",
                    id
                );
            }
            Err(err) => return Err(err),
        }
    }
    Ok(installed)
}

/// Refresh library symlinks after a pull: add new, remove stale under `repo_name/`.
pub fn refresh_library_symlinks(
    store: &StorePaths,
    repo_name: &str,
    skills: &[(String, PathBuf)],
) -> Result<(), SkmError> {
    let desired: std::collections::HashSet<String> =
        skills.iter().map(|(id, _)| id.clone()).collect();

    let existing = collect_repo_library_links(store, repo_name)?;
    for id in existing {
        if !desired.contains(&id) {
            let link = store.skill_dir(&id);
            if link.is_symlink() {
                fs::remove_file(&link)?;
                remove_remote_skill_meta(store, &id)?;
                if let Some(parent) = link.parent() {
                    prune_empty_parents(&store.skill_dir(repo_name), parent);
                }
            }
        }
    }

    install_library_symlinks(store, repo_name, skills)?;

    let repo_dir = store.skill_dir(repo_name);
    if repo_dir.is_dir() && fs::read_dir(&repo_dir)?.next().is_none() {
        fs::remove_dir(&repo_dir)?;
    }

    Ok(())
}

/// Symlink ids under `store/<repo_name>/` (filesystem scan, not skill discovery).
pub(crate) fn collect_repo_library_links(
    store: &StorePaths,
    repo_name: &str,
) -> Result<Vec<String>, SkmError> {
    let mut ids = Vec::new();
    let repo_root = store.skill_dir(repo_name);
    if repo_root.is_symlink() {
        ids.push(repo_name.to_string());
        return Ok(ids);
    }
    if repo_root.is_dir() {
        walk_library_symlink_ids(&repo_root, repo_name, &mut ids)?;
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

fn walk_library_symlink_ids(
    dir: &Path,
    id_prefix: &str,
    ids: &mut Vec<String>,
) -> Result<(), SkmError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let id = format!("{id_prefix}/{name}");
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            ids.push(id);
        } else if file_type.is_dir() {
            walk_library_symlink_ids(&path, &id, ids)?;
        }
    }
    Ok(())
}

/// Remove every library symlink for a registered remote.
pub fn remove_repo_library_links(store: &StorePaths, repo_name: &str) -> Result<(), SkmError> {
    let ids = collect_repo_library_links(store, repo_name)?;
    for id in ids {
        let link = store.skill_dir(&id);
        if link.is_symlink() {
            fs::remove_file(&link)?;
            if let Some(parent) = link.parent() {
                prune_empty_parents(&store.skill_dir(repo_name), parent);
            }
        }
    }

    let repo_dir = store.skill_dir(repo_name);
    if repo_dir.is_dir() && fs::read_dir(&repo_dir)?.next().is_none() {
        fs::remove_dir(&repo_dir)?;
    } else if repo_dir.is_symlink() {
        fs::remove_file(&repo_dir)?;
    }

    Ok(())
}

fn install_one_symlink(store: &StorePaths, id: &str, target: &Path) -> Result<(), SkmError> {
    let link_path = store.skill_dir(id);
    let target = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());

    if link_path.exists() {
        if link_path.is_symlink() {
            let current = fs::read_link(&link_path)?;
            let current_abs = if current.is_absolute() {
                current
            } else {
                link_path
                    .parent()
                    .unwrap_or(store.root())
                    .join(&current)
                    .canonicalize()
                    .unwrap_or(current)
            };
            let want = target.canonicalize().unwrap_or(target.clone());
            if current_abs == want {
                return Ok(());
            }
            fs::remove_file(&link_path)?;
        } else {
            return Err(SkmError::LibraryLinkOccupied {
                id: id.to_string(),
                path: link_path,
            });
        }
    }

    if let Some(parent) = link_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let rel = relative_path(link_path.parent().unwrap_or(store.root()), &target)?;
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&rel, &link_path)?;
    }
    #[cfg(not(unix))]
    {
        return Err(SkmError::Usage(
            "symlinks are not supported on this platform".to_string(),
        ));
    }
    Ok(())
}

fn prune_empty_parents(root: &Path, start: &Path) {
    let mut dir = Some(start);
    while let Some(current) = dir {
        if current == root {
            break;
        }
        let empty = fs::read_dir(current)
            .ok()
            .is_some_and(|mut e| e.next().is_none());
        if empty {
            let _ = fs::remove_dir(current);
            dir = current.parent();
        } else {
            break;
        }
    }
}

fn relative_path(from: &Path, to: &Path) -> Result<PathBuf, SkmError> {
    let from = from.canonicalize().unwrap_or_else(|_| from.to_path_buf());
    let to = to.canonicalize().unwrap_or_else(|_| to.to_path_buf());

    let mut from_components: Vec<_> = from.components().collect();
    let mut to_components: Vec<_> = to.components().collect();

    while !from_components.is_empty()
        && !to_components.is_empty()
        && from_components[0] == to_components[0]
    {
        from_components.remove(0);
        to_components.remove(0);
    }

    let mut result = PathBuf::new();
    for _ in from_components {
        result.push("..");
    }
    for comp in to_components {
        match comp {
            Component::CurDir => {}
            other => result.push(other.as_os_str()),
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{init_store_layout, StorePaths};
    use tempfile::TempDir;

    #[cfg(unix)]
    #[test]
    fn install_creates_relative_symlink() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let checkout = store.skm_dir().join("remotes/demo/skills/deploy");
        fs::create_dir_all(&checkout).unwrap();
        fs::write(checkout.join("SKILL.md"), "# deploy\n").unwrap();

        install_library_symlinks(
            &store,
            "demo",
            &[("demo/deploy".into(), checkout.clone())],
        )
        .unwrap();

        let link = store.skill_dir("demo/deploy");
        assert!(link.is_symlink());
        assert!(link.join("SKILL.md").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn refresh_removes_stale_library_symlink() {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        let checkout = store.skm_dir().join("remotes/demo/skills/deploy");
        let lint = store.skm_dir().join("remotes/demo/skills/lint");
        fs::create_dir_all(&checkout).unwrap();
        fs::create_dir_all(&lint).unwrap();
        fs::write(checkout.join("SKILL.md"), "# deploy\n").unwrap();
        fs::write(lint.join("SKILL.md"), "# lint\n").unwrap();

        install_library_symlinks(
            &store,
            "demo",
            &[
                ("demo/deploy".into(), checkout.clone()),
                ("demo/lint".into(), lint.clone()),
            ],
        )
        .unwrap();

        fs::remove_dir_all(&lint).unwrap();

        refresh_library_symlinks(
            &store,
            "demo",
            &[("demo/deploy".into(), checkout.clone())],
        )
        .unwrap();

        assert!(store.skill_dir("demo/deploy").is_symlink());
        assert!(!store.skill_dir("demo/lint").exists());
    }
}
