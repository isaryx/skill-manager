use clap::ValueEnum;

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
