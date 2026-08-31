use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::config::ProfileFile;
use crate::error::SkmError;
use crate::store::StorePaths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillPlacement {
    /// Store skill id (may be nested, e.g. `engineering/tdd`).
    pub store_id: String,
    /// Flat name under the agent skills directory (e.g. `tdd`).
    pub name: String,
    pub source: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("skill not found in store: {0}")]
    NotFound(String),
    #[error("resolve conflict for `{0}`: multiple candidates")]
    Conflict(String),
    #[error("profile is empty")]
    EmptyProfile,
}

impl From<ResolveError> for SkmError {
    fn from(err: ResolveError) -> Self {
        match err {
            ResolveError::NotFound(id) => SkmError::ResolveNotFound(id),
            ResolveError::Conflict(name) => SkmError::ResolveConflict(name),
            ResolveError::EmptyProfile => SkmError::EmptyProfile,
        }
    }
}

fn store_id_leaf(id: &str) -> &str {
    id.rsplit('/').next().unwrap_or(id)
}

fn store_id_to_flat_name(id: &str) -> String {
    id.replace('/', "__")
}

/// Map store ids to flat agent placement names.
///
/// Uses the leaf segment when unique (`engineering/tdd` → `tdd`). When multiple
/// store ids share the same leaf, disambiguates with `__` between segments (`a/tdd` → `a__tdd`).
pub fn assign_placement_names(store_ids: &[String]) -> Result<Vec<String>, ResolveError> {
    let mut leaf_counts: HashMap<&str, usize> = HashMap::new();
    for id in store_ids {
        *leaf_counts.entry(store_id_leaf(id)).or_insert(0) += 1;
    }

    let mut names = Vec::with_capacity(store_ids.len());
    let mut seen_names = HashSet::new();

    for id in store_ids {
        let name = if leaf_counts[store_id_leaf(id)] == 1 {
            store_id_leaf(id).to_string()
        } else {
            store_id_to_flat_name(id)
        };
        if !seen_names.insert(name.clone()) {
            return Err(ResolveError::Conflict(name));
        }
        names.push(name);
    }

    Ok(names)
}

/// Pure resolution: profile skill IDs → placement list.
pub fn resolve(
    profile: &ProfileFile,
    store: &StorePaths,
    disabled: &HashSet<String>,
) -> Result<Vec<SkillPlacement>, ResolveError> {
    if profile.skill.is_empty() {
        return Err(ResolveError::EmptyProfile);
    }

    let store_ids: Vec<String> = profile.skill.iter().map(|entry| entry.id.clone()).collect();

    let mut seen_store_ids = HashSet::new();
    for id in &store_ids {
        if !seen_store_ids.insert(id.clone()) {
            return Err(ResolveError::Conflict(id.clone()));
        }
    }

    let store_ids: Vec<String> = store_ids
        .into_iter()
        .filter(|id| !disabled.contains(id))
        .collect();
    if store_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placement_names = assign_placement_names(&store_ids)?;

    let mut placements = Vec::new();
    for (store_id, name) in store_ids.into_iter().zip(placement_names) {
        let source = store.skill_dir(&store_id);
        if !source.is_dir() || !source.join("SKILL.md").is_file() {
            return Err(ResolveError::NotFound(store_id));
        }

        placements.push(SkillPlacement {
            store_id,
            name,
            source: source.canonicalize().unwrap_or(source),
        });
    }

    Ok(placements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProfileSkillEntry;
    use crate::store::{init_store_layout, StorePaths};
    use std::fs;
    use tempfile::TempDir;

    fn make_store_with_skills(ids: &[&str]) -> (TempDir, StorePaths) {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();
        for id in ids {
            let dir = store.skill_dir(id);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("SKILL.md"), "# test").unwrap();
        }
        (tmp, store)
    }

    fn make_store_with_skill(id: &str) -> (TempDir, StorePaths) {
        make_store_with_skills(&[id])
    }

    #[test]
    fn resolves_known_skills() {
        let (_tmp, store) = make_store_with_skill("docx");
        let profile = ProfileFile {
            extends: Vec::new(),
            skill: vec![ProfileSkillEntry {
                id: "docx".to_string(),
            }],
        };
        let placements = resolve(&profile, &store, &HashSet::new()).unwrap();
        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].store_id, "docx");
        assert_eq!(placements[0].name, "docx");
    }

    #[test]
    fn nested_store_id_uses_leaf_placement_name() {
        let (_tmp, store) = make_store_with_skill("engineering/tdd");
        let profile = ProfileFile {
            extends: Vec::new(),
            skill: vec![ProfileSkillEntry {
                id: "engineering/tdd".to_string(),
            }],
        };
        let placements = resolve(&profile, &store, &HashSet::new()).unwrap();
        assert_eq!(placements[0].store_id, "engineering/tdd");
        assert_eq!(placements[0].name, "tdd");
    }

    #[test]
    fn disambiguates_shared_leaf_names() {
        let names =
            assign_placement_names(&["engineering/tdd".to_string(), "other/tdd".to_string()])
                .unwrap();
        assert_eq!(names, vec!["engineering__tdd", "other__tdd"]);
    }

    #[test]
    fn missing_skill_errors() {
        let (_tmp, store) = make_store_with_skill("docx");
        let profile = ProfileFile {
            extends: Vec::new(),
            skill: vec![ProfileSkillEntry {
                id: "missing".to_string(),
            }],
        };
        let err = resolve(&profile, &store, &HashSet::new()).unwrap_err();
        assert!(matches!(err, ResolveError::NotFound(_)));
    }

    #[test]
    fn disabled_skill_is_skipped() {
        let (_tmp, store) = make_store_with_skill("docx");
        let disabled = HashSet::from(["docx".to_string()]);
        let profile = ProfileFile {
            extends: Vec::new(),
            skill: vec![ProfileSkillEntry {
                id: "docx".to_string(),
            }],
        };
        let placements = resolve(&profile, &store, &disabled).unwrap();
        assert!(placements.is_empty());
    }

    #[test]
    fn disambiguation_avoids_hyphen_collisions() {
        let names =
            assign_placement_names(&["a/b/c".to_string(), "a-b/c".to_string(), "tdd".to_string()])
                .unwrap();
        assert_eq!(names, vec!["a__b__c", "a-b__c", "tdd"]);
    }

    #[test]
    fn flat_name_disambiguation_reports_collision_with_literal_id() {
        // "team/tdd" and "other/tdd" share the leaf "tdd" and disambiguate to
        // "team__tdd"/"other__tdd", which collides with the literal id "team__tdd" — a
        // genuine placement clash (both would need the same flat name in the agent
        // directory), not a false positive. Per docs/SPEC.md:107, an unresolvable
        // collision is expected to error.
        let ids = vec![
            "team/tdd".to_string(),
            "other/tdd".to_string(),
            "team__tdd".to_string(),
        ];
        let err = assign_placement_names(&ids).unwrap_err();
        assert!(matches!(err, ResolveError::Conflict(name) if name == "team__tdd"));
    }

    #[test]
    fn empty_profile_errors() {
        let (_tmp, store) = make_store_with_skill("docx");
        let profile = ProfileFile::default();
        let err = resolve(&profile, &store, &HashSet::new()).unwrap_err();
        assert!(matches!(err, ResolveError::EmptyProfile));
    }

    #[test]
    fn duplicate_store_ids_conflict() {
        let (_tmp, store) = make_store_with_skill("docx");
        let profile = ProfileFile {
            extends: Vec::new(),
            skill: vec![
                ProfileSkillEntry {
                    id: "docx".to_string(),
                },
                ProfileSkillEntry {
                    id: "docx".to_string(),
                },
            ],
        };
        let err = resolve(&profile, &store, &HashSet::new()).unwrap_err();
        assert!(matches!(err, ResolveError::Conflict(id) if id == "docx"));
    }

    #[test]
    fn disabled_peer_does_not_force_disambiguation() {
        let (_tmp, store) = make_store_with_skills(&["engineering/tdd", "other/tdd"]);
        let disabled = HashSet::from(["other/tdd".to_string()]);
        let profile = ProfileFile {
            extends: Vec::new(),
            skill: vec![
                ProfileSkillEntry {
                    id: "engineering/tdd".to_string(),
                },
                ProfileSkillEntry {
                    id: "other/tdd".to_string(),
                },
            ],
        };
        let placements = resolve(&profile, &store, &disabled).unwrap();
        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].store_id, "engineering/tdd");
        assert_eq!(placements[0].name, "tdd");
    }
}
