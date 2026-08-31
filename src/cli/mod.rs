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
    about = "Manage AI agent skills from one local store",
    long_about = "Keep canonical skill directories in a store, select them with profiles, and \
                  symlink the active profile into the agent skills directory (Claude Code, \
                  Cursor, or the generic Agent Skills layout).",
    after_help = "Examples:\n  \
                  skm init --agent claude-code\n  \
                  skm import ./my-skill --copy && skm profile setup work && skm use-profile work\n\n\
                  Pass --help for the full workflow, automation flags, and exit codes.",
    after_long_help = "Examples:\n  \
                       skm init --agent claude-code\n  \
                       skm import ./my-skill --copy\n  \
                       skm profile setup work && skm use-profile work\n\n\
                       Automation:\n  \
                       Select the store with SKM_STORE or --store.\n  \
                       --json works with `status`, `ls`, `skill ls`, and `doctor`; data goes to \
                       stdout, progress and errors to stderr.\n  \
                       --dry-run works with `sync`, `use-profile`, and `skill rm`.\n  \
                       Exit codes: 0 success, 1 runtime or health failure, 2 usage or resolve \
                       conflict.\n\n\
                       Docs and issues: https://github.com/isaryx/skill-manager"
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
    #[command(
        long_about = "Initialize the skill store and write project configuration to \
                      `./.skm.toml`.\n\n\
                      Existing project or hand-installed skills are preserved. Name conflicts \
                      are reported by `skm status` and `skm doctor`.",
        after_help = "Automation:\n  For non-interactive use, pass --agent. If the target agent \
                      directory already contains skills, also pass --accept-existing-skills."
    )]
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
    /// Import a skill directory into the store
    Import {
        /// Path to the skill directory
        dir: PathBuf,
        /// Copy the skill into the store (keeps the original)
        #[arg(long, conflicts_with = "move_")]
        copy: bool,
        /// Move the skill into the store (removes the original)
        #[arg(long = "move", conflicts_with = "copy")]
        move_: bool,
        /// Name to use in the store
        #[arg(long = "as", alias = "as-name")]
        as_name: Option<String>,
    },
    /// Create and manage profiles
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },
    /// Manage skills in the store
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
    /// Show agent, active profile, linked skills, and name conflicts
    #[command(
        long_about = "Read-only report of this project's skill wiring.\n\n\
                      Prints the target agent, active profile, Linked store-owned symlinks that \
                      match the profile, and Conflicts where a profile skill is blocked by a \
                      non-skm entry at that name.\n\n\
                      Does not create or repair links. Missing or broken links are not listed; \
                      use `skm doctor`.",
        after_help = "Requires `./.skm.toml` unless --user. Supports --json."
    )]
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
    /// Choose which profiles this one inherits skills from (interactive)
    #[command(
        long_about = "Pick the profiles whose skills this profile also includes, and which it \
                      therefore `extends`. Creates the profile if it does not exist, like \
                      `setup`.\n\n\
                      `extends` is a live reference: the skill list is flattened when the \
                      profile is synced, so later edits to a base profile apply here too. \
                      Profiles that already extend this one are not offered, since that would \
                      create a cycle.",
        after_help = "Chains are limited to 8 levels deep. Use `skm profile show` to see which \
                      profile each skill comes from."
    )]
    Extend {
        /// Profile name
        name: String,
    },
    /// List profile names
    Ls,
    /// Show skills in a profile
    #[command(
        long_about = "List the skills a profile resolves to, including everything it inherits \
                      through `extends`. Inherited skills are marked `(from <profile>)`.\n\n\
                      With --tree, print the extend graph instead: skills nest under the profile \
                      that declares them, and `(*)` marks a profile subtree or skill already \
                      accounted for above and so not counted twice. --tree \
                      also renders a broken graph, marking each `(cycle)`, `(not found)`, \
                      `(too deep)` or `(unreadable)` in place, then exits with the same code the \
                      flat listing would.",
        after_help = "Stdout is one skill per line; --tree replaces that with the tree and its \
                      skill count."
    )]
    Show {
        /// Profile name
        name: String,
        /// Print the extend graph as a tree instead of a flat skill list
        #[arg(long)]
        tree: bool,
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
    /// List enabled skills in the store
    Ls,
    /// Choose which store skills are enabled (interactive; all enabled by default)
    Setup,
    /// Permanently remove a skill from the store
    #[command(
        long_about = "Permanently remove a skill from the store and update profiles that \
                      reference it.",
        after_help = "Automation:\n  Non-interactive use requires --force. Use --dry-run to \
                      inspect the affected profiles and links first."
    )]
    Rm {
        /// Store skill ID to remove
        id: String,
        /// Remove without confirmation (required when stdin is not a TTY)
        #[arg(long)]
        force: bool,
    },
}
