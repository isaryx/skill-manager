use clap::ValueEnum;

use crate::adapters::get_adapter;
use crate::error::SkmError;

#[derive(Clone, Debug, ValueEnum)]
pub enum Agent {
    #[value(name = "claude-code", help = "Claude Code (.claude/skills)")]
    ClaudeCode,
    #[value(help = "Cursor (.cursor/skills)")]
    Cursor,
    #[value(
        name = "generic",
        help = "Agent Skills (.agents/skills); Codex, Cursor, Gemini CLI, Copilot CLI"
    )]
    Generic,
    #[value(name = "gemini-cli", help = "Gemini CLI (.gemini/skills)")]
    GeminiCli,
    #[value(
        name = "copilot-cli",
        help = "Copilot CLI (.github/skills; ~/.copilot/skills with --user)"
    )]
    CopilotCli,
}

/// The ids behind `--agent`, validated and with repeats dropped.
///
/// `--agent` is repeatable, so `--agent cursor --agent cursor` is easy to type; the config
/// should say `cursor` once.
pub fn unique_agent_ids(agents: &[Agent]) -> Result<Vec<String>, SkmError> {
    let mut ids: Vec<String> = Vec::with_capacity(agents.len());
    for agent in agents {
        get_adapter(agent.as_str())?;
        let id = agent.as_str().to_string();
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    Ok(ids)
}

impl Agent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Agent::ClaudeCode => "claude-code",
            Agent::Cursor => "cursor",
            Agent::Generic => "generic",
            Agent::GeminiCli => "gemini-cli",
            Agent::CopilotCli => "copilot-cli",
        }
    }
}
