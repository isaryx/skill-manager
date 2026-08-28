use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::color::ColorWhen;

pub mod agent;
pub mod doctor;
pub mod import;
pub mod init;
pub mod ls;
pub mod output;
pub mod profile;
pub mod scan;
pub mod skill;
pub mod status;
pub mod switch_agent;
pub mod sync;
pub mod use_cmd;

pub use agent::Agent;

#[derive(Parser, Debug)]
#[command(
    name = "skm",
    version,
    about = "Manage AI agent skills from one local library",
    long_about = "Keep skills in one store, organize them into profiles, and link them into \
                  agent folders (Claude Code, Cursor, Codex, Gemini CLI, Copilot CLI, generic). \
                  Run `skm sync` to refresh links."
)]
pub struct Cli {
    /// Enable verbose logging on stderr
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Store root directory (env: SKM_STORE)
    #[arg(long, global = true, env = "SKM_STORE")]
    pub store: Option<PathBuf>,

    /// Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)
    #[arg(long, global = true)]
    pub json: bool,

    /// Preview changes without writing (`sync`, `use-profile`, `skill rm` only)
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// When to colorize human output (`auto` respects NO_COLOR)
    #[arg(long, global = true, value_enum, default_value_t = ColorWhen::Auto)]
    pub color: ColorWhen,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Set up the skill store and project config (`.skm.toml`)
    Init {
        /// Target agent for this project
        #[arg(long)]
        agent: Option<Agent>,
        /// Overwrite an existing `.skm.toml`
        #[arg(long)]
        force: bool,
        /// Proceed when the agent skills directory already has entries (non-interactive)
        #[arg(long = "accept-existing-skills")]
        accept_existing_skills: bool,
    },
    /// Import a skill directory into the library
    #[command(visible_aliases = ["add"])]
    Import {
        /// Path to the skill directory
        dir: PathBuf,
        /// Copy the skill into the library (keeps the original)
        #[arg(long, conflicts_with = "move_")]
        copy: bool,
        /// Move the skill into the library (removes the original)
        #[arg(long = "move", conflicts_with = "copy")]
        move_: bool,
        /// Name to use in the library
        #[arg(long = "as", alias = "as-name")]
        as_name: Option<String>,
    },
    /// Create and manage profiles
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },
    /// Manage skills in the library
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    /// Activate a profile and sync skill links
    #[command(name = "use-profile")]
    UseProfile {
        /// Profile name
        profile: String,
        /// Use `~/.skm.toml` even when `./.skm.toml` exists
        #[arg(short = 'u', long)]
        user: bool,
    },
    /// Change the target agent in your config
    #[command(name = "switch-agent")]
    SwitchAgent {
        /// Agent to switch to
        #[arg(long)]
        agent: Option<Agent>,
        /// Use `~/.skm.toml` even when `./.skm.toml` exists
        #[arg(short = 'u', long)]
        user: bool,
    },
    /// Refresh skill links without changing the active profile
    Sync {
        /// Use `~/.skm.toml` even when `./.skm.toml` exists
        #[arg(short = 'u', long)]
        user: bool,
    },
    /// Show the target agent, active profile, linked skills, and placement conflicts
    Status {
        /// Use `~/.skm.toml` even when `./.skm.toml` exists
        #[arg(short = 'u', long)]
        user: bool,
    },
    /// List skills and profiles in the store
    Ls {
        /// List profiles only (same as `skm profile ls`)
        #[arg(short = 'p', long = "profile", conflicts_with = "skill")]
        profile: bool,
        /// List skills only (same as `skm skill ls`)
        #[arg(short = 's', long = "skill", conflicts_with = "profile")]
        skill: bool,
    },
    /// Refresh the on-disk skill index
    Scan,
    /// Read-only health report for the store, profiles, and skill links
    Doctor {
        /// Use `~/.skm.toml` even when `./.skm.toml` exists
        #[arg(short = 'u', long)]
        user: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProfileAction {
    /// Choose skills for a profile (interactive; creates the profile if missing)
    Setup {
        /// Profile name
        name: String,
    },
    /// List profile names
    Ls,
    /// Show skills in a profile
    Show {
        /// Profile name
        name: String,
        /// Use user-level config when checking which profile is active
        #[arg(short = 'u', long)]
        user: bool,
    },
    /// Remove a profile
    Rm {
        /// Profile name
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum SkillAction {
    /// List enabled skills in the library
    Ls,
    /// Choose which library skills are enabled (interactive; all enabled by default)
    Setup,
    /// Permanently remove a skill from the library
    Rm {
        /// Store skill ID to remove
        id: String,
        /// Remove without confirmation (required when stdin is not a TTY)
        #[arg(long)]
        force: bool,
    },
}
