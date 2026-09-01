use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

use dialoguer::{Confirm, Input};

use crate::config::home_dir;
use crate::error::SkmError;
use crate::tui::{MultiSelect, MultiSelectItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupLevel {
    User,
    Project,
}

pub trait AgentAdapter {
    fn name(&self) -> &'static str;
    fn target_dir(&self, level: SetupLevel, project_root: &Path) -> PathBuf;
}

pub struct ClaudeCodeAdapter;

impl AgentAdapter for ClaudeCodeAdapter {
    fn name(&self) -> &'static str {
        "claude-code"
    }

    fn target_dir(&self, level: SetupLevel, project_root: &Path) -> PathBuf {
        match level {
            SetupLevel::User => home_dir().join(".claude").join("skills"),
            SetupLevel::Project => project_root.join(".claude").join("skills"),
        }
    }
}

pub struct CursorAdapter;

impl AgentAdapter for CursorAdapter {
    fn name(&self) -> &'static str {
        "cursor"
    }

    fn target_dir(&self, level: SetupLevel, project_root: &Path) -> PathBuf {
        match level {
            SetupLevel::User => home_dir().join(".cursor").join("skills"),
            SetupLevel::Project => project_root.join(".cursor").join("skills"),
        }
    }
}

/// Agents that document `.agents/skills` as a native or interoperable skills path.
pub const GENERIC_AGENT_CLIENTS: &str = "Codex, Cursor, Gemini CLI, Copilot CLI";

/// [Agent Skills](https://agentskills.io) layout (`.agents/skills`). Used natively by Codex;
/// Cursor, Gemini CLI, and Copilot CLI also read this path as an interoperable alias.
pub struct GenericAdapter;

impl AgentAdapter for GenericAdapter {
    fn name(&self) -> &'static str {
        "generic"
    }

    fn target_dir(&self, level: SetupLevel, project_root: &Path) -> PathBuf {
        match level {
            SetupLevel::User => home_dir().join(".agents").join("skills"),
            SetupLevel::Project => project_root.join(".agents").join("skills"),
        }
    }
}

pub struct GeminiCliAdapter;

impl AgentAdapter for GeminiCliAdapter {
    fn name(&self) -> &'static str {
        "gemini-cli"
    }

    fn target_dir(&self, level: SetupLevel, project_root: &Path) -> PathBuf {
        match level {
            SetupLevel::User => home_dir().join(".gemini").join("skills"),
            SetupLevel::Project => project_root.join(".gemini").join("skills"),
        }
    }
}

pub struct CopilotCliAdapter;

impl AgentAdapter for CopilotCliAdapter {
    fn name(&self) -> &'static str {
        "copilot-cli"
    }

    fn target_dir(&self, level: SetupLevel, project_root: &Path) -> PathBuf {
        match level {
            SetupLevel::User => home_dir().join(".copilot").join("skills"),
            SetupLevel::Project => project_root.join(".github").join("skills"),
        }
    }
}

pub fn get_adapter(name: &str) -> Result<Box<dyn AgentAdapter>, SkmError> {
    match name {
        "claude-code" => Ok(Box::new(ClaudeCodeAdapter)),
        "cursor" => Ok(Box::new(CursorAdapter)),
        "generic" | "codex" => Ok(Box::new(GenericAdapter)),
        "gemini-cli" => Ok(Box::new(GeminiCliAdapter)),
        "copilot-cli" => Ok(Box::new(CopilotCliAdapter)),
        other => Err(SkmError::UnknownAgent(other.to_string())),
    }
}

/// One agent and the skills directory it places into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTarget {
    pub agent: String,
    pub dir: PathBuf,
}

pub fn resolve_target_dir(
    agent: &str,
    level: SetupLevel,
    project_root: &Path,
) -> Result<AgentTarget, SkmError> {
    let adapter = get_adapter(agent)?;
    Ok(AgentTarget {
        agent: agent.to_string(),
        dir: adapter.target_dir(level, project_root),
    })
}

/// Resolve every agent to its skills directory, keeping the caller's order and dropping
/// agents that repeat a directory already covered.
///
/// Two ids can name the same directory (`codex` is an alias of `generic`), and visiting it
/// twice would have the second pass unwire what the first had just wired.
pub fn resolve_target_dirs(
    agents: &[String],
    level: SetupLevel,
    project_root: &Path,
) -> Result<Vec<AgentTarget>, SkmError> {
    let mut targets: Vec<AgentTarget> = Vec::with_capacity(agents.len());
    for agent in agents {
        let target = resolve_target_dir(agent, level, project_root)?;
        if !targets.iter().any(|seen| seen.dir == target.dir) {
            targets.push(target);
        }
    }
    Ok(targets)
}

/// Prompt once when any target agent's skills directory already holds entries.
///
/// The check is per directory rather than per agent so that two agents sharing a directory are
/// only mentioned once.
pub fn confirm_agent_skills_dirs_if_nonempty(
    agents: &[String],
    project_root: &Path,
    accept_existing: bool,
) -> Result<(), SkmError> {
    let mut occupied = Vec::new();
    for target in resolve_target_dirs(agents, SetupLevel::Project, project_root)? {
        if !target.dir.is_dir() {
            continue;
        }
        if fs::read_dir(&target.dir)?.next().is_some() {
            occupied.push(target);
        }
    }

    if occupied.is_empty() {
        return Ok(());
    }

    if accept_existing {
        crate::progress::step(
            "agent skills directories have existing entries; skm will only manage its own \
             symlinks",
        );
        return Ok(());
    }

    let listed = occupied
        .iter()
        .map(|target| format!("{} ({})", target.agent, target.dir.display()))
        .collect::<Vec<_>>()
        .join(", ");

    if !io::stdin().is_terminal() {
        return Err(SkmError::Usage(format!(
            "agent skills directory is not empty: {listed}; pass --accept-existing-skills to \
             proceed (skm will not remove project or hand-installed skills)"
        )));
    }

    let proceed = Confirm::new()
        .with_prompt(format!(
            "These agent skills directories already contain skills: {listed}.\n\
             skm only manages its own symlinks and will not remove project or hand-installed \
             skills. Names already taken will be skipped during sync.\n\
             Continue?"
        ))
        .default(false)
        .interact()
        .map_err(|_| SkmError::SelectionCancelled)?;

    if proceed {
        Ok(())
    } else {
        Err(SkmError::SelectionCancelled)
    }
}

pub fn interactive_select_store_location(default: &Path) -> Result<PathBuf, SkmError> {
    if !io::stdin().is_terminal() {
        return Err(SkmError::NotATty);
    }

    let input: String = Input::new()
        .with_prompt("Skill store location")
        .default(default.display().to_string())
        .interact_text()
        .map_err(|_| SkmError::SelectionCancelled)?;

    let path = input.trim();
    if path.is_empty() {
        return Err(SkmError::Usage(
            "store location cannot be empty".to_string(),
        ));
    }

    Ok(PathBuf::from(path))
}

const INIT_AGENTS: &[&str] = &[
    "generic",
    "claude-code",
    "cursor",
    "gemini-cli",
    "copilot-cli",
];

/// Canonical ids shown in the agent picker and cleaned by `skm destroy`.
pub fn known_agent_ids() -> &'static [&'static str] {
    INIT_AGENTS
}

/// Maps legacy config ids to their canonical menu / CLI id.
pub fn canonical_agent_id(agent: &str) -> &str {
    match agent {
        "codex" => "generic",
        other => other,
    }
}

pub fn interactive_select_agents(
    preselected: &[String],
    level: SetupLevel,
    project_root: &Path,
) -> Result<Vec<String>, SkmError> {
    if !io::stdin().is_terminal() {
        return Err(SkmError::NotATty);
    }

    let preselected: Vec<&str> = preselected
        .iter()
        .map(|agent| canonical_agent_id(agent))
        .collect();

    let mut items = Vec::with_capacity(INIT_AGENTS.len());
    for &agent in INIT_AGENTS {
        items.push(
            MultiSelectItem::new(agent)
                .hint(agent_hint(agent, level, project_root)?)
                .selected(preselected.contains(&agent)),
        );
    }

    let chosen = MultiSelect::new("Target agents").items(items).interact()?;

    if chosen.is_empty() {
        return Err(SkmError::Usage(
            "no agent selected; pick at least one with space".to_string(),
        ));
    }

    Ok(chosen)
}

pub(crate) fn format_agent_skills_path(
    agent: &str,
    level: SetupLevel,
    project_root: &Path,
) -> Result<String, SkmError> {
    let adapter = get_adapter(agent)?;
    let path = adapter.target_dir(level, project_root);
    if level == SetupLevel::User {
        let home = home_dir();
        if let Ok(rel) = path.strip_prefix(&home) {
            return Ok(format!("~/{}", rel.to_string_lossy()));
        }
    }
    Ok(path
        .strip_prefix(project_root)
        .unwrap_or(&path)
        .to_string_lossy()
        .trim_start_matches("./")
        .to_string())
}

/// Secondary text for an agent's row: where it places skills, and for the interoperable
/// `.agents/skills` layout, which clients read it.
fn agent_hint(agent: &str, level: SetupLevel, project_root: &Path) -> Result<String, SkmError> {
    let path = format_agent_skills_path(agent, level, project_root)?;
    Ok(match agent {
        "generic" => format!("{path} — {GENERIC_AGENT_CLIENTS}"),
        _ => path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_hint_shows_project_skills_path() {
        let hint = agent_hint("generic", SetupLevel::Project, Path::new("/tmp/proj")).unwrap();
        assert!(hint.contains(".agents/skills"));
    }

    #[test]
    fn agent_hint_shows_user_skills_path() {
        let home = home_dir();
        let hint = agent_hint("claude-code", SetupLevel::User, &home).unwrap();
        assert!(hint.contains("~/.claude/skills"));
    }

    #[test]
    fn codex_canonicalizes_to_generic() {
        assert_eq!(canonical_agent_id("codex"), "generic");
        assert_eq!(canonical_agent_id("cursor"), "cursor");
    }

    #[test]
    fn known_agent_ids_all_resolve() {
        for agent in known_agent_ids() {
            get_adapter(agent).unwrap_or_else(|_| panic!("{agent}"));
        }
        let targets = resolve_target_dirs(
            &known_agent_ids()
                .iter()
                .map(|agent| (*agent).to_string())
                .collect::<Vec<_>>(),
            SetupLevel::Project,
            Path::new("/tmp/proj"),
        )
        .unwrap();
        assert_eq!(targets.len(), known_agent_ids().len());
    }

    #[test]
    fn resolve_target_dirs_drops_agents_sharing_a_directory() {
        let agents = vec![
            "generic".to_string(),
            "codex".to_string(),
            "cursor".to_string(),
        ];
        let targets =
            resolve_target_dirs(&agents, SetupLevel::Project, Path::new("/tmp/proj")).unwrap();
        assert_eq!(
            targets
                .iter()
                .map(|target| target.agent.as_str())
                .collect::<Vec<_>>(),
            vec!["generic", "cursor"]
        );
    }

    #[test]
    fn resolve_target_dirs_rejects_unknown_agent() {
        let agents = vec!["windsurf".to_string()];
        assert!(matches!(
            resolve_target_dirs(&agents, SetupLevel::Project, Path::new("/tmp/proj")),
            Err(SkmError::UnknownAgent(_))
        ));
    }

    #[test]
    fn cursor_uses_cursor_skills_path() {
        let adapter = CursorAdapter;
        let project = Path::new("/tmp/proj");
        assert_eq!(
            adapter.target_dir(SetupLevel::Project, project),
            project.join(".cursor/skills")
        );
        let home = home_dir();
        assert_eq!(
            adapter.target_dir(SetupLevel::User, project),
            home.join(".cursor/skills")
        );
    }

    #[test]
    fn codex_config_alias_uses_generic_adapter() {
        let adapter = get_adapter("codex").unwrap();
        let project = Path::new("/tmp/proj");
        assert_eq!(adapter.name(), "generic");
        assert_eq!(
            adapter.target_dir(SetupLevel::Project, project),
            project.join(".agents/skills")
        );
        let home = home_dir();
        assert_eq!(
            adapter.target_dir(SetupLevel::User, project),
            home.join(".agents/skills")
        );
    }

    #[test]
    fn agent_hint_notes_generic_clients() {
        let hint = agent_hint("generic", SetupLevel::Project, Path::new("/tmp/proj")).unwrap();
        assert!(hint.contains("Codex"));
        assert!(hint.contains("Cursor"));
        assert!(hint.contains("Gemini CLI"));
        assert!(hint.contains("Copilot CLI"));
    }

    #[test]
    fn gemini_cli_uses_gemini_skills_path() {
        let adapter = GeminiCliAdapter;
        let project = Path::new("/tmp/proj");
        assert_eq!(
            adapter.target_dir(SetupLevel::Project, project),
            project.join(".gemini/skills")
        );
    }

    #[test]
    fn copilot_cli_uses_github_skills_path() {
        let adapter = CopilotCliAdapter;
        let project = Path::new("/tmp/proj");
        assert_eq!(
            adapter.target_dir(SetupLevel::Project, project),
            project.join(".github/skills")
        );
        let home = home_dir();
        assert_eq!(
            adapter.target_dir(SetupLevel::User, project),
            home.join(".copilot/skills")
        );
    }
}
