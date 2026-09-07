use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::Serialize;

use crate::error::SkmError;
use crate::progress;

const MAX_NAME_LEN: usize = 64;
const MAX_DESCRIPTION_LEN: usize = 1024;
const MAX_COMPATIBILITY_LEN: usize = 500;

#[derive(Debug, Clone, Serialize)]
pub struct SkillValidationReport {
    pub path: String,
    pub valid: bool,
    pub issues: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct SkillFrontmatter {
    name: Option<String>,
    description: Option<String>,
    compatibility: Option<String>,
}

/// Validate a skill directory (or a path to `SKILL.md`) against the Agent Skills spec.
pub fn validate_skill_spec(path: &Path) -> SkillValidationReport {
    let skill_dir = resolve_skill_root(path);
    let issues = spec_issues(&skill_dir);
    SkillValidationReport {
        path: skill_dir.display().to_string(),
        valid: issues.is_empty(),
        issues,
    }
}

/// Validate every skill directory and warn or fail depending on `strict`.
pub fn check_skill_specs(skill_dirs: &[PathBuf], strict: bool) -> Result<(), SkmError> {
    let reports = skill_dirs
        .iter()
        .map(|dir| validate_skill_spec(dir))
        .filter(|report| !report.valid)
        .collect::<Vec<_>>();

    for report in &reports {
        for issue in &report.issues {
            progress::warn(format!("{}: {}", report.path, issue));
        }
    }

    if strict && !reports.is_empty() {
        let paths = reports
            .iter()
            .map(|report| report.path.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(SkmError::SkillSpecInvalid(paths));
    }

    Ok(())
}

fn resolve_skill_root(path: &Path) -> PathBuf {
    if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md") {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf())
    } else {
        path.to_path_buf()
    }
}

fn spec_issues(skill_dir: &Path) -> Vec<String> {
    let skill_file = skill_dir.join("SKILL.md");
    if !skill_file.is_file() {
        return vec![format!("SKILL.md not found in {}", skill_dir.display())];
    }

    let frontmatter = match parse_skill_md(&skill_file) {
        Ok(frontmatter) => frontmatter,
        Err(err) => return vec![err],
    };

    validate_frontmatter(&frontmatter, skill_dir)
}

fn parse_skill_md(path: &Path) -> Result<SkillFrontmatter, String> {
    let content =
        fs::read_to_string(path).map_err(|err| format!("reading {}: {}", path.display(), err))?;

    if !content.starts_with("---\n") {
        return Err(format!(
            "{}: missing YAML frontmatter (must start with ---)",
            path.display()
        ));
    }

    let rest = &content[4..];
    let end = rest
        .find("\n---")
        .ok_or_else(|| format!("{}: missing closing --- for frontmatter", path.display()))?;
    let yaml = &rest[..end];

    serde_yaml::from_str(yaml)
        .map_err(|err| format!("{}: invalid frontmatter YAML: {}", path.display(), err))
}

/// Read the searchable description from a skill's YAML frontmatter.
///
/// Invalid or legacy skill files remain indexable by ID and simply have no description.
pub fn read_description(skill_dir: &Path) -> Option<String> {
    parse_skill_md(&skill_dir.join("SKILL.md"))
        .ok()
        .and_then(|frontmatter| frontmatter.description)
}

fn validate_frontmatter(frontmatter: &SkillFrontmatter, skill_dir: &Path) -> Vec<String> {
    let mut issues = Vec::new();
    let dir_name = skill_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    match frontmatter.name.as_deref() {
        None | Some("") => issues.push("name: required field is missing".to_string()),
        Some(name) => {
            if name != dir_name {
                issues.push(format!(
                    "name: \"{name}\" does not match directory name \"{dir_name}\""
                ));
            }
            if name.len() > MAX_NAME_LEN {
                issues.push("name: exceeds 64 characters".to_string());
            }
            if name.contains("--") {
                issues.push("name: must not contain consecutive hyphens".to_string());
            }
            if !is_valid_spec_name(name) {
                issues.push(format!(
                    "name: \"{name}\" must be lowercase letters, numbers, and single hyphens"
                ));
            }
        }
    }

    match frontmatter.description.as_deref() {
        None | Some("") => issues.push("description: required field is missing".to_string()),
        Some(description) if description.len() > MAX_DESCRIPTION_LEN => {
            issues.push("description: exceeds 1024 characters".to_string());
        }
        _ => {}
    }

    if let Some(compatibility) = frontmatter.compatibility.as_deref() {
        if compatibility.is_empty() {
            issues.push("compatibility: must not be empty when present".to_string());
        } else if compatibility.len() > MAX_COMPATIBILITY_LEN {
            issues.push("compatibility: exceeds 500 characters".to_string());
        }
    }

    issues
}

/// `^[a-z][a-z0-9]*(-[a-z0-9]+)*$` from the Agent Skills / myskills validators.
fn is_valid_spec_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(ch) if ch.is_ascii_lowercase() => {}
        _ => return false,
    }

    while let Some(ch) = chars.next() {
        if ch == '-' {
            match chars.next() {
                Some(next) if next.is_ascii_lowercase() || next.is_ascii_digit() => {}
                _ => return false,
            }
        } else if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_skill_md(dir: &Path, contents: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("SKILL.md"), contents).unwrap();
    }

    #[test]
    fn valid_skill_passes() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("demo-skill");
        write_skill_md(
            &skill,
            "---\nname: demo-skill\ndescription: Does demo things when you need a demo.\n---\n",
        );

        let report = validate_skill_spec(&skill);
        assert!(report.valid, "issues: {:?}", report.issues);
    }

    #[test]
    fn missing_frontmatter_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("demo");
        write_skill_md(&skill, "# demo\n");

        let report = validate_skill_spec(&skill);
        assert!(!report.valid);
        assert!(report.issues[0].contains("missing YAML frontmatter"));
    }

    #[test]
    fn name_must_match_directory() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("demo");
        write_skill_md(
            &skill,
            "---\nname: other\ndescription: A long enough description for the demo skill.\n---\n",
        );

        let report = validate_skill_spec(&skill);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.contains("does not match directory")));
    }

    #[test]
    fn description_is_required() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("demo");
        write_skill_md(&skill, "---\nname: demo\n---\n");

        let report = validate_skill_spec(&skill);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue == "description: required field is missing"));
    }

    #[test]
    fn accepts_path_to_skill_md_file() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("demo");
        write_skill_md(
            &skill,
            "---\nname: demo\ndescription: Validates when the argument is SKILL.md itself.\n---\n",
        );

        let report = validate_skill_spec(&skill.join("SKILL.md"));
        assert!(report.valid);
    }

    #[test]
    fn reads_description_for_indexing() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("demo");
        write_skill_md(
            &skill,
            "---\nname: demo\ndescription: Finds deployment documentation.\n---\n",
        );

        assert_eq!(
            read_description(&skill).as_deref(),
            Some("Finds deployment documentation.")
        );
    }

    #[test]
    fn strict_mode_fails_when_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("demo");
        write_skill_md(&skill, "# no frontmatter\n");

        assert!(check_skill_specs(&[skill.clone()], false).is_ok());
        assert!(matches!(
            check_skill_specs(&[skill], true),
            Err(SkmError::SkillSpecInvalid(_))
        ));
    }
}
