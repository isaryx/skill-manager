use std::collections::HashMap;

use serde::Serialize;

use crate::db::{open_index, search_skills};
use crate::error::SkmError;
use crate::progress;
use crate::store::extends::flatten_skill_ids;
use crate::store::profiles::list_profiles;
use crate::store::StorePaths;

const DESCRIPTION_WIDTH: usize = 80;

#[derive(Debug, Serialize)]
struct SearchResult {
    id: String,
    enabled: bool,
    profiles: Vec<String>,
    description: String,
}

pub fn run_search(store: &StorePaths, terms: &[String], json: bool) -> Result<(), SkmError> {
    store.ensure_initialized()?;
    let rows = search_skills(&open_index(store)?, terms)?;
    let memberships = profile_memberships(store)?;
    let results: Vec<SearchResult> = rows
        .into_iter()
        .map(|row| SearchResult {
            profiles: memberships.get(&row.id).cloned().unwrap_or_default(),
            id: row.id,
            enabled: row.enabled,
            description: row.description,
        })
        .collect();

    if json {
        return super::output::write_json(&results)
            .map_err(|err| SkmError::Usage(format!("failed to encode JSON: {err}")));
    }

    if results.is_empty() {
        progress::step("no matching skills");
        progress::step("run `skm scan` if you recently edited SKILL.md files on disk");
        return Ok(());
    }

    for result in results {
        let state = if result.enabled {
            "enabled"
        } else {
            "disabled"
        };
        let profiles = if result.profiles.is_empty() {
            "-".to_string()
        } else {
            result.profiles.join(",")
        };
        println!(
            "{}\t{}\t{}\t{}",
            result.id,
            state,
            profiles,
            truncate_description(&result.description)
        );
    }
    Ok(())
}

fn profile_memberships(store: &StorePaths) -> Result<HashMap<String, Vec<String>>, SkmError> {
    let mut memberships: HashMap<String, Vec<String>> = HashMap::new();
    for profile_name in list_profiles(store)? {
        let skill_ids = match flatten_skill_ids(store, &profile_name) {
            Ok(ids) => ids,
            Err(err) => {
                progress::warn(format!(
                    "skipping profile `{profile_name}` in search results: {err}"
                ));
                continue;
            }
        };
        for skill_id in skill_ids {
            memberships
                .entry(skill_id)
                .or_default()
                .push(profile_name.clone());
        }
    }
    for profiles in memberships.values_mut() {
        profiles.sort();
    }
    Ok(memberships)
}

fn truncate_description(description: &str) -> String {
    let normalized = description.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= DESCRIPTION_WIDTH {
        return normalized;
    }
    let prefix: String = normalized.chars().take(DESCRIPTION_WIDTH - 1).collect();
    format!("{prefix}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn description_is_normalized_and_truncated_on_character_boundaries() {
        let description = format!("  {}  {}", "é".repeat(79), "tail");
        let truncated = truncate_description(&description);

        assert_eq!(truncated.chars().count(), DESCRIPTION_WIDTH);
        assert!(truncated.ends_with('…'));
    }
}
