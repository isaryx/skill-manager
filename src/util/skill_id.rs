use crate::error::SkmError;
use crate::util::validate_store_entry_name;

/// Combine a remote repo name with a skill path segment (`deploy` → `agent-skills/deploy`).
pub fn qualify_skill_id(repo: &str, skill: &str) -> Result<String, SkmError> {
    validate_store_entry_name(repo)?;
    if skill.is_empty() {
        return Ok(repo.to_string());
    }
    crate::util::validate_store_skill_id(skill)?;
    Ok(format!("{repo}/{skill}"))
}

/// First path segment of a nested store id (repo name or bundle root).
pub fn id_first_segment(id: &str) -> Option<&str> {
    id.split_once('/').map(|(segment, _)| segment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualify_prefixes_skill_with_repo() {
        assert_eq!(
            qualify_skill_id("agent-skills", "deploy").unwrap(),
            "agent-skills/deploy"
        );
    }

    #[test]
    fn qualify_rejects_invalid_repo() {
        assert!(qualify_skill_id("../evil", "deploy").is_err());
    }

    #[test]
    fn id_first_segment_extracts_leading_path_segment() {
        assert_eq!(
            id_first_segment("agent-skills/deploy"),
            Some("agent-skills")
        );
        assert_eq!(id_first_segment("engineering/tdd"), Some("engineering"));
        assert_eq!(id_first_segment("docx"), None);
    }
}
