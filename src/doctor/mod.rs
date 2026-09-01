mod checks;

use std::env;

use serde::Serialize;

use crate::error::SkmError;
use crate::progress::{display_path, step};
use crate::setup::select_setup_lenient;
use crate::store::StorePaths;

pub use checks::{Issue, Severity};

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub ok: bool,
    pub store: String,
    pub agents: Vec<String>,
    pub profile: Option<String>,
    pub issues: Vec<Issue>,
}

impl Report {
    pub fn exit_code(&self) -> i32 {
        if self.ok {
            0
        } else {
            1
        }
    }
}

pub fn run_checks(store: &StorePaths, force_user: bool) -> Result<Report, SkmError> {
    let cwd = env::current_dir()?;
    let store_display = display_path(store.root());

    let selected = select_setup_lenient(&cwd, force_user).ok();
    let mut issues = Vec::new();

    step("checking skill store");
    issues.extend(checks::check_store(store));

    if store.is_initialized() {
        issues.extend(checks::check_index(store)?);
        issues.extend(checks::check_skills_on_disk(store)?);
        issues.extend(checks::check_meta(store)?);
        issues.extend(checks::check_profiles(store)?);
    }

    let (agents, profile) = if let Some(ref selected) = selected {
        step("checking profiles");
        issues.extend(checks::check_config(selected));

        let agents = selected.setup.placement.resolved_agents();
        let profile = selected.setup.profile.active.clone();

        // Every agent has to resolve before the link check runs: it walks all of their
        // directories at once, and an unknown agent is already reported by `check_config`.
        let agents_known = !agents.is_empty()
            && agents
                .iter()
                .all(|agent| crate::adapters::get_adapter(agent).is_ok());

        if let Some(active) = profile.as_deref() {
            if agents_known {
                step("checking links");
                issues.extend(checks::check_links(store, selected, active)?);
            }
        }

        (agents, profile)
    } else {
        (Vec::new(), None)
    };

    let ok = !issues.iter().any(|issue| {
        matches!(
            issue.severity,
            checks::Severity::Warn | checks::Severity::Error
        )
    });

    Ok(Report {
        ok,
        store: store_display,
        agents,
        profile,
        issues,
    })
}
