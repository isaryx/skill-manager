use clap::ValueEnum;

#[derive(Clone, Debug, ValueEnum)]
pub enum Agent {
    #[value(name = "claude-code")]
    ClaudeCode,
    Cursor,
    #[value(
        name = "generic",
        help = "Agent Skills layout (.agents/skills); used by Codex, Cursor, Gemini CLI, Copilot CLI"
    )]
    Generic,
    #[value(name = "gemini-cli")]
    GeminiCli,
    #[value(name = "copilot-cli")]
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
