use std::fs;
use std::path::{Path, PathBuf};

use crate::error::SkmError;
use crate::util::is_path_inside;

pub(crate) fn resolve_symlink_target(path: &Path) -> Option<PathBuf> {
    let target = fs::read_link(path).ok()?;
    let resolved = if target.is_absolute() {
        target
    } else {
        path.parent()?.join(target)
    };
    Some(resolved.canonicalize().unwrap_or(resolved))
}

pub(crate) fn is_store_owned_symlink(path: &Path, store_root: &Path) -> bool {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return false;
    };
    if !meta.file_type().is_symlink() {
        return false;
    }
    resolve_symlink_target(path).is_some_and(|resolved| is_path_inside(store_root, &resolved))
}

/// Entry exists and is not a symlink owned by skm (project skill, hand-installed file, etc.).
pub(crate) fn is_foreign_occupant(path: &Path, store_root: &Path) -> bool {
    path.exists() && !is_store_owned_symlink(path, store_root)
}

fn relative_name(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}

pub(crate) fn walk_store_owned_symlinks<F>(
    target: &Path,
    store_root: &Path,
    mut visitor: F,
) -> Result<(), SkmError>
where
    F: FnMut(&Path, String) -> Result<(), SkmError>,
{
    fn walk(
        base: &Path,
        dir: &Path,
        store_root: &Path,
        visitor: &mut dyn FnMut(&Path, String) -> Result<(), SkmError>,
    ) -> Result<(), SkmError> {
        if !dir.is_dir() {
            return Ok(());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let rel = relative_name(base, &path);
            if rel.is_empty() {
                continue;
            }

            if is_store_owned_symlink(&path, store_root) {
                visitor(&path, rel)?;
            } else if path.is_dir() {
                walk(base, &path, store_root, visitor)?;
            }
        }
        Ok(())
    }

    walk(target, target, store_root, &mut visitor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn missing_path_is_not_foreign() {
        let store = TempDir::new().unwrap();
        let path = store.path().join("missing");
        assert!(!is_foreign_occupant(&path, store.path()));
    }

    #[test]
    fn file_occupant_is_foreign() {
        let store = TempDir::new().unwrap();
        let skills = store.path().join("skills");
        fs::create_dir_all(&skills).unwrap();
        let file = skills.join("docx");
        fs::write(&file, "blocked").unwrap();

        assert!(!is_store_owned_symlink(&file, store.path()));
        assert!(is_foreign_occupant(&file, store.path()));
    }

    #[test]
    fn directory_occupant_is_foreign() {
        let store = TempDir::new().unwrap();
        let skills = store.path().join("skills");
        let dir = skills.join("review");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), "# review\n").unwrap();

        assert!(!is_store_owned_symlink(&dir, store.path()));
        assert!(is_foreign_occupant(&dir, store.path()));
    }

    #[cfg(unix)]
    #[test]
    fn store_owned_symlink_is_not_foreign() {
        let store = TempDir::new().unwrap();
        let skill = store.path().join("docx");
        fs::create_dir_all(&skill).unwrap();
        let skills = store.path().join("agent-skills");
        fs::create_dir_all(&skills).unwrap();
        let link = skills.join("docx");
        std::os::unix::fs::symlink(&skill, &link).unwrap();

        let store_root = store.path().canonicalize().unwrap();
        assert!(is_store_owned_symlink(&link, &store_root));
        assert!(!is_foreign_occupant(&link, &store_root));
    }

    #[cfg(unix)]
    #[test]
    fn outside_store_symlink_is_foreign() {
        let store = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let skill = outside.path().join("docx");
        fs::create_dir_all(&skill).unwrap();
        let skills = store.path().join("agent-skills");
        fs::create_dir_all(&skills).unwrap();
        let link = skills.join("docx");
        std::os::unix::fs::symlink(&skill, &link).unwrap();

        let store_root = store.path().canonicalize().unwrap();
        assert!(!is_store_owned_symlink(&link, &store_root));
        assert!(is_foreign_occupant(&link, &store_root));
    }
}
