use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::error::SkmError;

/// Validate a single path segment: `[a-z0-9][a-z0-9._-]*`
pub fn validate_skill_id(id: &str) -> Result<(), SkmError> {
    validate_name(id, |name| SkmError::InvalidSkillId(name.to_string()))
}

/// Validate a store skill id (single segment or nested path like `engineering/tdd`).
pub fn validate_store_skill_id(id: &str) -> Result<(), SkmError> {
    if id.is_empty() || id.contains("..") || id.starts_with('/') || id.ends_with('/') {
        return Err(SkmError::InvalidSkillId(id.to_string()));
    }
    for segment in id.split('/') {
        validate_skill_id(segment)?;
    }
    Ok(())
}

/// Validate a single-segment store entry name (bundle root or top-level skill).
pub fn validate_store_entry_name(name: &str) -> Result<(), SkmError> {
    validate_skill_id(name)
}

/// Profile names use the same rules as single-segment skill IDs.
pub fn validate_profile_name(name: &str) -> Result<(), SkmError> {
    validate_name(name, |n| SkmError::InvalidProfileName(n.to_string()))
}

fn validate_name(name: &str, invalid: impl FnOnce(&str) -> SkmError) -> Result<(), SkmError> {
    if name.is_empty() {
        return Err(invalid(name));
    }
    if name == ".skm" || name.starts_with('.') {
        return Err(SkmError::ReservedName(name.to_string()));
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(invalid(name));
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(invalid(name));
    }
    for c in chars {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '.' && c != '_' && c != '-' {
            return Err(invalid(name));
        }
    }
    Ok(())
}

/// True when `child` is `base` or a path under `base` (not a string-prefix sibling).
pub fn is_path_inside(base: &Path, child: &Path) -> bool {
    let base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
    let child = child.canonicalize().unwrap_or_else(|_| child.to_path_buf());
    if child == base {
        return true;
    }
    child
        .strip_prefix(&base)
        .is_ok_and(|rel| matches!(rel.components().next(), Some(Component::Normal(_))))
}

/// Content hash of a directory tree (sorted paths, sha256 of each file).
pub fn hash_directory(path: &Path) -> Result<String, SkmError> {
    let mut hasher = Sha256::new();
    let mut files: Vec<PathBuf> = WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();
    files.sort();

    for file in files {
        let rel = file
            .strip_prefix(path)
            .map_err(|e| SkmError::Io(std::io::Error::other(e.to_string())))?;
        hasher.update(rel.as_os_str().as_encoded_bytes());
        let mut f = fs::File::open(&file)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        hasher.update(&buf);
    }

    let digest = hasher.finalize();
    let hex = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    Ok(format!("sha256:{hex}"))
}

/// Recursively copy a directory. Rejects symlinks in the source tree.
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), SkmError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let meta = fs::symlink_metadata(&src_path)?;
        if meta.file_type().is_symlink() {
            return Err(SkmError::SymlinkInSkillTree(src_path));
        }
        let dst_path = dst.join(entry.file_name());
        if meta.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Check whether `path` is a skill directory (has SKILL.md at root).
pub fn is_skill_dir(path: &Path) -> bool {
    path.is_dir() && path.join("SKILL.md").is_file()
}

/// Every directory under `root` that contains SKILL.md (any nesting depth).
pub fn discover_all_skill_dirs(root: &Path) -> Result<Vec<PathBuf>, SkmError> {
    let mut skills = Vec::new();
    if root.is_dir() {
        discover_skill_dirs_walk(root, &mut skills)?;
    }
    skills.sort();
    skills.dedup();
    Ok(skills)
}

fn discover_skill_dirs_walk(dir: &Path, skills: &mut Vec<PathBuf>) -> Result<(), SkmError> {
    if dir.file_name().and_then(|n| n.to_str()) == Some(".skm") {
        return Ok(());
    }

    if is_skill_dir(dir) {
        skills.push(dir.to_path_buf());
    }

    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            discover_skill_dirs_walk(&entry.path(), skills)?;
        }
    }
    Ok(())
}

/// True when `path` is a directory tree that contains skills other than `path` itself alone.
pub fn is_skill_tree(path: &Path) -> bool {
    match discover_all_skill_dirs(path) {
        Ok(skills) => {
            if skills.is_empty() {
                return false;
            }
            if skills.len() > 1 {
                return true;
            }
            !same_path(&skills[0], path)
        }
        Err(_) => false,
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    fs::canonicalize(a).ok() == fs::canonicalize(b).ok() || a == b
}

pub(crate) fn paths_equal(a: &Path, b: &Path) -> bool {
    same_path(a, b)
}

/// A bundle is a directory that contains skills at any nesting depth but is not itself
/// a skill root (no root SKILL.md).
pub fn is_skill_bundle(path: &Path) -> bool {
    if is_skill_dir(path) || !path.is_dir() {
        return false;
    }
    discover_all_skill_dirs(path)
        .map(|dirs| !dirs.is_empty())
        .unwrap_or(false)
}

/// Immediate child directories that each contain SKILL.md at their root.
pub fn list_immediate_skill_dirs(parent: &Path) -> Result<Vec<PathBuf>, SkmError> {
    let mut skills = Vec::new();
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() && is_skill_dir(&path) {
            skills.push(path);
        }
    }
    skills.sort();
    Ok(skills)
}

/// Relative store skill id from a path under the store root.
pub fn path_to_store_skill_id(root: &Path, skill_path: &Path) -> Option<String> {
    let rel = skill_path.strip_prefix(root).ok()?;
    let id = rel.to_string_lossy().replace('\\', "/");
    validate_store_skill_id(&id).ok()?;
    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    #[test]
    fn is_path_inside_rejects_prefix_siblings() {
        let tmp = TempDir::new().unwrap();
        let store = tmp.path().join("store");
        let sibling = tmp.path().join("store-evil");
        let docx = store.join("docx");
        fs::create_dir_all(&docx).unwrap();
        fs::create_dir_all(&sibling).unwrap();
        assert!(!is_path_inside(&store, &sibling));
        assert!(is_path_inside(&store, &docx));
    }

    #[test]
    fn copy_dir_all_rejects_symlinks() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("SKILL.md"), "# x").unwrap();
        symlink(tmp.path().join("outside"), src.join("link")).unwrap();

        let err = copy_dir_all(&src, &dst).unwrap_err();
        assert!(matches!(err, SkmError::SymlinkInSkillTree(_)));
    }

    #[test]
    fn profile_name_rejects_traversal() {
        assert!(validate_profile_name("../outside").is_err());
        assert!(validate_profile_name("infra").is_ok());
    }

    #[test]
    fn store_skill_id_allows_nested_paths() {
        assert!(validate_store_skill_id("engineering/tdd").is_ok());
        assert!(validate_store_skill_id("docx").is_ok());
        assert!(validate_store_skill_id("../tdd").is_err());
        assert!(validate_store_skill_id("engineering//tdd").is_err());
        assert!(validate_store_skill_id("Docx").is_err());
    }

    #[test]
    fn reserved_and_leading_dot_names_rejected() {
        assert!(matches!(
            validate_store_entry_name(".skm"),
            Err(SkmError::ReservedName(_))
        ));
        assert!(matches!(
            validate_store_entry_name(".hidden"),
            Err(SkmError::ReservedName(_))
        ));
    }

    #[test]
    fn hash_directory_is_stable_for_same_tree() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("skill");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), "# x\n").unwrap();
        let a = hash_directory(&dir).unwrap();
        let b = hash_directory(&dir).unwrap();
        assert_eq!(a, b);
        assert!(a.starts_with("sha256:"));
    }

    #[test]
    fn bundle_lists_immediate_skill_children() {
        let tmp = TempDir::new().unwrap();
        let bundle = tmp.path().join("engineering");
        fs::create_dir_all(bundle.join("tdd")).unwrap();
        fs::create_dir_all(bundle.join("code-review")).unwrap();
        fs::write(bundle.join("README.md"), "# bundle").unwrap();
        fs::write(bundle.join("tdd/SKILL.md"), "# tdd").unwrap();
        fs::write(bundle.join("code-review/SKILL.md"), "# cr").unwrap();

        assert!(!is_skill_dir(&bundle));
        assert!(is_skill_bundle(&bundle));
        let children = list_immediate_skill_dirs(&bundle).unwrap();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn discover_all_skill_dirs_finds_any_depth() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("tree");
        let deep = root.join("a/b/c/skill");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("SKILL.md"), "# deep").unwrap();

        let skills = discover_all_skill_dirs(&root).unwrap();
        assert_eq!(skills.len(), 1);
        assert!(skills[0].ends_with("a/b/c/skill"));
        assert!(is_skill_tree(&root));
    }

    #[test]
    fn is_skill_bundle_recognizes_multi_level_nesting() {
        let tmp = TempDir::new().unwrap();
        let vendor = tmp.path().join("vendor");
        let deep = vendor.join("team/tdd");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("SKILL.md"), "# tdd").unwrap();

        assert!(
            is_skill_bundle(&vendor),
            "a directory containing a skill nested 2+ levels deep should still be recognized as a bundle"
        );
    }

    #[test]
    fn skill_tree_detects_nested_skill_under_skill() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("parent");
        fs::create_dir_all(root.join("child")).unwrap();
        fs::write(root.join("SKILL.md"), "# parent").unwrap();
        fs::write(root.join("child/SKILL.md"), "# child").unwrap();

        let skills = discover_all_skill_dirs(&root).unwrap();
        assert_eq!(skills.len(), 2);
        assert!(is_skill_tree(&root));
    }
}
