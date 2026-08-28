use std::env;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

use dialoguer::console::{Key, Term};
use dialoguer::{Confirm, Input, Select};

use crate::config::home_dir;
use crate::error::SkmError;

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

pub struct CodexAdapter;

impl AgentAdapter for CodexAdapter {
    fn name(&self) -> &'static str {
        "codex"
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
        "generic" => Ok(Box::new(GenericAdapter)),
        "codex" => Ok(Box::new(CodexAdapter)),
        "gemini-cli" => Ok(Box::new(GeminiCliAdapter)),
        "copilot-cli" => Ok(Box::new(CopilotCliAdapter)),
        other => Err(SkmError::UnknownAgent(other.to_string())),
    }
}

pub fn resolve_target_dir(
    agent: &str,
    level: SetupLevel,
    project_root: &Path,
) -> Result<(String, PathBuf), SkmError> {
    let adapter = get_adapter(agent)?;
    Ok((agent.to_string(), adapter.target_dir(level, project_root)))
}

pub fn confirm_agent_skills_dir_if_nonempty(
    agent: &str,
    project_root: &Path,
    accept_existing: bool,
) -> Result<(), SkmError> {
    let adapter = get_adapter(agent)?;
    let dir = adapter.target_dir(SetupLevel::Project, project_root);
    if !dir.is_dir() {
        return Ok(());
    }

    let mut entries = fs::read_dir(&dir)?;
    if entries.next().is_none() {
        return Ok(());
    }

    if accept_existing {
        crate::progress::step(
            "agent skills directory has existing entries; skm will only manage its own symlinks",
        );
        return Ok(());
    }

    if !io::stdin().is_terminal() {
        return Err(SkmError::Usage(format!(
            "{} skills directory is not empty ({path}); pass --accept-existing-skills to proceed \
             (skm will not remove project or hand-installed skills)",
            agent,
            path = dir.display()
        )));
    }

    let proceed = Confirm::new()
        .with_prompt(format!(
            "The {agent} skills directory ({path}) already contains skills.\n\
             skm only manages its own symlinks and will not remove project or hand-installed \
             skills. Names already taken will be skipped during sync.\n\
             Continue with init?",
            path = dir.display()
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
    "codex",
    "gemini-cli",
    "copilot-cli",
];

pub fn interactive_select_agent() -> Result<String, SkmError> {
    let cwd = env::current_dir()?;
    interactive_select_agent_impl("Target agent", SetupLevel::Project, &cwd)
}

pub fn interactive_switch_agent(
    current: &str,
    level: SetupLevel,
    project_root: &Path,
) -> Result<String, SkmError> {
    interactive_switch_agent_select(current, level, project_root)
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

fn agent_select_label(
    agent: &str,
    level: SetupLevel,
    project_root: &Path,
) -> Result<String, SkmError> {
    let path = format_agent_skills_path(agent, level, project_root)?;
    Ok(format!("{agent} \x1b[2m({path})\x1b[0m"))
}

fn interactive_select_agent_impl(
    prompt: &str,
    level: SetupLevel,
    project_root: &Path,
) -> Result<String, SkmError> {
    if !io::stdin().is_terminal() {
        return Err(SkmError::NotATty);
    }

    let labels: Vec<String> = INIT_AGENTS
        .iter()
        .map(|&agent| agent_select_label(agent, level, project_root))
        .collect::<Result<_, _>>()?;

    let selection = Select::new()
        .with_prompt(prompt)
        .items(&labels)
        .default(0)
        .interact_opt()
        .map_err(|_| SkmError::SelectionCancelled)?;

    match selection {
        Some(index) => Ok(INIT_AGENTS[index].to_string()),
        None => Err(SkmError::SelectionCancelled),
    }
}

fn interactive_switch_agent_select(
    current: &str,
    level: SetupLevel,
    project_root: &Path,
) -> Result<String, SkmError> {
    if !io::stdin().is_terminal() {
        return Err(SkmError::NotATty);
    }

    let current_index = INIT_AGENTS
        .iter()
        .position(|&agent| agent == current)
        .ok_or_else(|| SkmError::UnknownAgent(current.to_string()))?;

    let labels: Vec<String> = INIT_AGENTS
        .iter()
        .map(|&agent| {
            let mut label = agent_select_label(agent, level, project_root)?;
            if agent == current {
                label.push_str(" \x1b[2m(current)\x1b[0m");
            }
            Ok(label)
        })
        .collect::<Result<_, SkmError>>()?;

    let prompt = "Target agent";
    let term = Term::stderr();
    let mut sel = current_index;
    let line_count = labels.len() + 1;

    term.hide_cursor()
        .map_err(|_| SkmError::SelectionCancelled)?;

    loop {
        term.write_line(&format!("{prompt}:"))
            .map_err(|_| SkmError::SelectionCancelled)?;

        for (idx, label) in labels.iter().enumerate() {
            let prefix = if sel == idx { "> " } else { "  " };
            term.write_line(&format!("{prefix}{label}"))
                .map_err(|_| SkmError::SelectionCancelled)?;
        }
        term.flush().ok();

        match term.read_key().map_err(|_| SkmError::SelectionCancelled)? {
            Key::ArrowDown | Key::Tab | Key::Char('j') => {
                sel = (sel + 1) % labels.len();
            }
            Key::ArrowUp | Key::BackTab | Key::Char('k') => {
                sel = (sel + labels.len() - 1) % labels.len();
            }
            Key::Escape | Key::Char('q') => {
                term.show_cursor().ok();
                return Err(SkmError::SelectionCancelled);
            }
            Key::Enter | Key::Char(' ') if sel != current_index => {
                term.clear_last_lines(line_count).ok();
                term.show_cursor().ok();
                return Ok(INIT_AGENTS[sel].to_string());
            }
            Key::Enter | Key::Char(' ') => {}
            _ => {}
        }

        term.clear_last_lines(line_count).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_select_label_shows_project_skills_path() {
        let label =
            agent_select_label("generic", SetupLevel::Project, Path::new("/tmp/proj")).unwrap();
        assert!(label.contains("generic"));
        assert!(label.contains(".agents/skills"));
    }

    #[test]
    fn agent_select_label_shows_user_skills_path() {
        let home = home_dir();
        let label = agent_select_label("claude-code", SetupLevel::User, &home).unwrap();
        assert!(label.contains("claude-code"));
        assert!(label.contains("~/.claude/skills"));
    }

    #[test]
    fn switch_agent_label_marks_current_agent() {
        let labels: Vec<String> = INIT_AGENTS
            .iter()
            .map(|&agent| {
                let mut label =
                    agent_select_label(agent, SetupLevel::Project, Path::new("/tmp/proj")).unwrap();
                if agent == "cursor" {
                    label.push_str(" \x1b[2m(current)\x1b[0m");
                }
                label
            })
            .collect();

        let cursor_idx = INIT_AGENTS.iter().position(|&a| a == "cursor").unwrap();
        assert!(labels[cursor_idx].contains("(current)"));
        assert!(!labels[0].contains("(current)"));
    }

    #[test]
    fn codex_uses_agents_skills_path() {
        let adapter = CodexAdapter;
        let project = Path::new("/tmp/proj");
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
