use std::path::{Path, PathBuf};

use crate::error::SkmError;
use crate::util::{discover_all_skill_dirs, is_skill_dir, list_immediate_skill_dirs};

/// Standard skill directory names searched in order at the checkout root.
pub const STANDARD_SKILL_DIRS: &[&str] = &[
    "skills",
    ".agents/skills",
    ".cursor/skills",
    ".github/skills",
    ".claude/skills",
    ".copilot/skills",
];

/// Result of skill root discovery: absolute path and store-relative segment (`""` for root skill).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillsRoot {
    pub path: PathBuf,
    /// Relative to checkout root; empty when the repo itself is a single skill.
    pub segment: String,
}

/// Find the skills root inside a git checkout.
pub fn find_skills_root(checkout: &Path) -> Result<Option<SkillsRoot>, SkmError> {
    for segment in STANDARD_SKILL_DIRS {
        let candidate = checkout.join(segment);
        if candidate.is_dir() {
            let children = list_immediate_skill_dirs(&candidate)?;
            if !children.is_empty() {
                return Ok(Some(SkillsRoot {
                    path: candidate,
                    segment: segment.to_string(),
                }));
            }
        }
    }

    if is_skill_dir(checkout) {
        return Ok(Some(SkillsRoot {
            path: checkout.to_path_buf(),
            segment: String::new(),
        }));
    }

    Ok(None)
}

/// List repo-qualified skill ids and their absolute paths under a checkout.
pub fn list_repo_skills(
    _checkout: &Path,
    skills_root: &SkillsRoot,
    repo_name: &str,
) -> Result<Vec<(String, PathBuf)>, SkmError> {
    if skills_root.segment.is_empty() {
        return Ok(vec![(repo_name.to_string(), skills_root.path.clone())]);
    }

    let mut out = Vec::new();
    for skill_path in discover_all_skill_dirs(&skills_root.path)? {
        let rel_within_root = skill_path
            .strip_prefix(&skills_root.path)
            .map_err(|e| SkmError::Io(std::io::Error::other(e.to_string())))?;
        let rel_str = rel_within_root.to_string_lossy().replace('\\', "/");
        let id = if rel_str.is_empty() {
            repo_name.to_string()
        } else {
            format!("{repo_name}/{rel_str}")
        };
        out.push((id, skill_path));
    }

    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.dedup_by(|a, b| a.0 == b.0);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_skill(dir: &Path, name: &str) {
        let skill = dir.join(name);
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), format!("# {name}\n")).unwrap();
    }

    #[test]
    fn discovers_skills_directory_first() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write_skill(&root.join("skills"), "deploy");
        write_skill(root, "root-skill");

        let found = find_skills_root(root).unwrap().unwrap();
        assert_eq!(found.segment, "skills");
        assert!(found.path.ends_with("skills"));
    }

    #[test]
    fn root_skill_when_no_standard_dir_matches() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join("SKILL.md"), "# root\n").unwrap();

        let found = find_skills_root(root).unwrap().unwrap();
        assert!(found.segment.is_empty());
        assert_eq!(found.path, root);
    }

    #[test]
    fn list_repo_skills_under_skills_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write_skill(&root.join("skills"), "deploy");
        write_skill(&root.join("skills/nested"), "foo");

        let skills_root = SkillsRoot {
            path: root.join("skills"),
            segment: "skills".into(),
        };
        let skills = list_repo_skills(root, &skills_root, "myskills").unwrap();
        let ids: Vec<&str> = skills.iter().map(|(id, _)| id.as_str()).collect();
        assert!(ids.contains(&"myskills/deploy"));
        assert!(ids.contains(&"myskills/nested/foo"));
    }

    #[test]
    fn list_repo_skills_root_skill_repo() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join("SKILL.md"), "# solo\n").unwrap();

        let skills_root = SkillsRoot {
            path: root.to_path_buf(),
            segment: String::new(),
        };
        let skills = list_repo_skills(root, &skills_root, "solo-repo").unwrap();
        assert_eq!(skills, vec![("solo-repo".to_string(), root.to_path_buf())]);
    }
}
