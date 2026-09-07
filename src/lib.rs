pub mod adapters;
pub mod cli;
pub mod color;
pub mod config;
pub mod db;
pub mod doctor;
pub mod error;
pub mod progress;
pub mod resolver;
pub mod setup;
pub mod store;
pub mod sync;
pub mod tui;
pub mod util;

use std::env;

use crate::cli::ls::LsFilter;
use crate::cli::repo::{run_repo_add, run_repo_ls};
use crate::cli::sync as sync_cmd;
use crate::cli::update::run_update;
use crate::cli::{
    destroy, doctor::run_doctor, import, init, ls, profile, scan, search, skill, status,
    use_agents, use_cmd, Commands, ProfileAction, RepoAction, SkillAction,
};
use crate::config::resolve_store_root;
use crate::store::StorePaths;
use crate::sync::ReconcileOptions;

pub use cli::{cli_command, Cli};
pub use error::{exit_code_from_error, print_error, SkmError};

pub fn run(cli: Cli) -> Result<i32, SkmError> {
    crate::color::init(cli.color);
    validate_global_flags(&cli)?;

    let store = StorePaths::new(resolve_store_root(cli.store.as_deref())?);
    let json = cli.json;
    let dry_run = cli.dry_run;
    let reconcile_opts = ReconcileOptions {
        dry_run,
        strict: false,
        no_pull: false,
    };

    let exit = match cli.command {
        Commands::Init {
            agent,
            force,
            accept_existing_skills,
        } => {
            init::run_init(cli.store.as_deref(), &agent, force, accept_existing_skills)?;
            0
        }
        Commands::Import {
            dir,
            copy,
            move_,
            as_name,
            repo,
            strict,
        } => {
            import::run_import(&store, &dir, copy, move_, as_name, repo, strict)?;
            0
        }
        Commands::Profile { action } => {
            match action {
                ProfileAction::Setup { name } => profile::run_profile_setup(&store, &name)?,
                ProfileAction::Extend { name } => profile::run_profile_extend(&store, &name)?,
                ProfileAction::Ls => profile::run_profile_ls(&store)?,
                ProfileAction::Show { name, user, tree } => {
                    profile::run_profile_show(&store, &name, user, tree)?
                }
                ProfileAction::Rm { name } => profile::run_profile_rm(&store, &name)?,
            }
            0
        }
        Commands::Skill { action } => {
            match action {
                SkillAction::Ls => skill::run_ls(&store, json)?,
                SkillAction::Setup => skill::run_setup(&store)?,
                SkillAction::Rm { id, force } => skill::run_rm(&store, &id, force, dry_run)?,
                SkillAction::Validate { path } => {
                    let path = skill::resolve_validate_path(&store, &path);
                    return skill::run_validate(&path, json);
                }
            }
            0
        }
        Commands::UseProfiles { user } => {
            use_cmd::run_use_profiles(&store, user)?;
            0
        }
        Commands::AddProfile { profile, user } => {
            use_cmd::run_add_profile(&store, &profile, user, reconcile_opts)?;
            0
        }
        Commands::RemoveProfile { profile, user } => {
            use_cmd::run_remove_profile(&store, &profile, user, reconcile_opts)?;
            0
        }
        Commands::Sync {
            user,
            no_pull,
            strict,
        } => {
            sync_cmd::run_sync(
                &store,
                user,
                ReconcileOptions {
                    dry_run,
                    strict,
                    no_pull,
                },
            )?;
            0
        }
        Commands::UseAgents { user } => {
            use_agents::run_use_agents(&store, user)?;
            0
        }
        Commands::AddAgent { agent, user } => {
            use_agents::run_add_agent(&store, &agent, user)?;
            0
        }
        Commands::RemoveAgent { agent, user } => {
            use_agents::run_remove_agent(&store, &agent, user)?;
            0
        }
        Commands::Destroy { force } => {
            destroy::run_destroy(&store, force, dry_run)?;
            0
        }
        Commands::Status { user } => {
            status::run_status(&store, user, json)?;
            0
        }
        Commands::Ls { profile, skill } => {
            let filter = if profile {
                LsFilter::Profiles
            } else if skill {
                LsFilter::Skills
            } else {
                LsFilter::All
            };
            ls::run(&store, filter, json)?;
            0
        }
        Commands::Scan => {
            scan::run_scan(&store)?;
            0
        }
        Commands::Search { query } => {
            search::run_search(&store, &query, json)?;
            0
        }
        Commands::Doctor { user } => run_doctor(&store, user, json)?,
        Commands::Repo { action } => {
            match action {
                RepoAction::Add {
                    r#ref,
                    name,
                    strict,
                } => run_repo_add(&store, &r#ref, name.as_deref(), strict)?,
                RepoAction::Ls => run_repo_ls(&store, json)?,
            }
            0
        }
        Commands::Update { name } => {
            run_update(&store, name.as_deref(), dry_run)?;
            0
        }
    };

    Ok(exit)
}

fn validate_global_flags(cli: &Cli) -> Result<(), SkmError> {
    if cli.dry_run && cli.json {
        return Err(SkmError::Usage(
            "--dry-run cannot be used with --json".into(),
        ));
    }
    if cli.json && !supports_json(&cli.command) {
        return Err(SkmError::Usage(
            "--json is only supported for status, ls, search, skill ls, skill validate, doctor, and repo ls"
                .into(),
        ));
    }
    if cli.dry_run && !supports_dry_run(&cli.command) {
        return Err(SkmError::Usage(
            "--dry-run is only supported for sync, update, add-profile, remove-profile, skill rm, and destroy".into(),
        ));
    }
    Ok(())
}

fn supports_json(command: &Commands) -> bool {
    matches!(
        command,
        Commands::Status { .. }
            | Commands::Ls { .. }
            | Commands::Search { .. }
            | Commands::Doctor { .. }
    ) || matches!(
        command,
        Commands::Skill {
            action: SkillAction::Ls | SkillAction::Validate { .. },
        }
    ) || matches!(
        command,
        Commands::Repo {
            action: RepoAction::Ls,
        }
    )
}

fn supports_dry_run(command: &Commands) -> bool {
    matches!(
        command,
        Commands::Sync { .. }
            | Commands::Update { .. }
            | Commands::AddProfile { .. }
            | Commands::RemoveProfile { .. }
            | Commands::Destroy { .. }
    ) || matches!(
        command,
        Commands::Skill {
            action: SkillAction::Rm { .. },
        }
    )
}

pub fn init_logging(verbose: bool) {
    let level = if verbose { "debug" } else { "warn" };
    env::set_var("RUST_LOG", format!("skill_manager={level}"));
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level))
        .format_timestamp(None)
        .init();
}
