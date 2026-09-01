use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SkmError {
    #[error("store not initialized; run `skm init`")]
    StoreNotInitialized,

    #[error(
        "config file already exists: {0}; change agents with `skm setup-agents`; activate a \
         profile with `skm use-profile <name>`"
    )]
    SetupExists(PathBuf),

    #[error("config file not found: {0}")]
    SetupNotFound(PathBuf),

    #[error("application config not found: {0}")]
    AppConfigNotFound(PathBuf),

    #[error("home directory not found")]
    HomeNotFound,

    #[error("invalid application config {path}: {message}")]
    InvalidAppConfig { path: PathBuf, message: String },

    #[error("invalid skill id: {0}")]
    InvalidSkillId(String),

    #[error("invalid profile name: {0}")]
    InvalidProfileName(String),

    #[error("reserved name: {0}")]
    ReservedName(String),

    #[error("destination already exists: {0}")]
    DestinationExists(PathBuf),

    #[error("skill directory must contain SKILL.md at its root, or one or more SKILL.md files in subdirectories: {0}")]
    NotASkillDir(PathBuf),

    #[error("no skill directories found under {0}")]
    EmptySkillBundle(PathBuf),

    #[error("skill tree must not contain symlinks: {0}")]
    SymlinkInSkillTree(PathBuf),

    #[error("unknown agent: {0}")]
    UnknownAgent(String),

    #[error("config lists no target agents; run `skm setup-agents`")]
    NoTargetAgents,

    #[error("profile not found: {0}")]
    ProfileNotFound(String),

    #[error("no profiles available; create one with `skm profile setup <name>`")]
    NoProfiles,

    #[error("profile is empty; add skills with `skm profile setup`")]
    EmptyProfile,

    #[error("no active profile; run `skm use-profile <profile>`")]
    NoActiveProfile,

    #[error("cannot remove active profile `{0}`; switch profiles first")]
    ActiveProfileRemoval(String),

    #[error("duplicate skill id in profile: {0}")]
    DuplicateSkillId(String),

    #[error("profile `{0}` cannot extend itself")]
    SelfExtend(String),

    #[error("duplicate extended profile: {0}")]
    DuplicateExtend(String),

    #[error("extend cycle between profiles: {0}")]
    ExtendCycle(String),

    #[error("extend chain is deeper than {limit}: {chain}")]
    ExtendTooDeep { limit: usize, chain: String },

    #[error("profile `{profile}` extends missing profile `{missing}`")]
    ExtendNotFound { profile: String, missing: String },

    #[error("cannot remove profile `{profile}`; extended by {extenders}")]
    ExtendedProfileRemoval { profile: String, extenders: String },

    #[error(
        "no profiles available to extend; a profile cannot extend itself, nor anything that \
         already extends it"
    )]
    NoExtendCandidates,

    #[error("skill library is empty; import skills with `skm import`")]
    EmptyPool,

    #[error("interactive mode requires a TTY; pass --agent <agent> and/or --store <path>")]
    NotATty,

    #[error("invalid skill store at {path}: {message}")]
    InvalidStore { path: PathBuf, message: String },

    #[error("profile selection cancelled")]
    SelectionCancelled,

    #[error("invalid profile file {path}: {message}")]
    InvalidProfile { path: PathBuf, message: String },

    #[error("invalid config file {path}: {message}")]
    InvalidSetup { path: PathBuf, message: String },

    #[error("resolve conflict for `{0}`: multiple candidates")]
    ResolveConflict(String),

    #[error("skill referenced by profile not found in store: {0}")]
    ResolveNotFound(String),

    #[error("skill not found: {0}")]
    SkillNotFound(String),

    #[error("refusing to remove without --force")]
    RefuseNonInteractiveRm,

    #[error("refusing to destroy without --force")]
    RefuseNonInteractiveDestroy,

    #[error("{0}")]
    Usage(String),

    #[error("{operation}")]
    WithContext {
        operation: String,
        #[source]
        source: Box<SkmError>,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    #[error(transparent)]
    TomlSerialize(#[from] toml::ser::Error),

    #[error(transparent)]
    TomlDeserialize(#[from] toml::de::Error),
}

impl SkmError {
    pub fn exit_code(&self) -> i32 {
        match self {
            SkmError::WithContext { source, .. } => source.exit_code(),
            SkmError::Usage(_)
            | SkmError::DuplicateSkillId(_)
            | SkmError::ResolveConflict(_)
            | SkmError::InvalidProfileName(_)
            | SkmError::InvalidSkillId(_)
            | SkmError::SelfExtend(_)
            | SkmError::DuplicateExtend(_)
            | SkmError::ExtendCycle(_)
            | SkmError::ExtendTooDeep { .. }
            | SkmError::RefuseNonInteractiveRm
            | SkmError::RefuseNonInteractiveDestroy => 2,
            _ => 1,
        }
    }

    pub fn op(self, operation: impl Into<String>) -> Self {
        SkmError::WithContext {
            operation: operation.into(),
            source: Box::new(self),
        }
    }

    pub fn leaf(&self) -> &SkmError {
        match self {
            SkmError::WithContext { source, .. } => source.leaf(),
            other => other,
        }
    }
}

pub fn exit_code_from_error(err: &SkmError) -> i32 {
    err.exit_code()
}

/// Print a user-facing error: leaf domain message by default, full chain with `--verbose`.
pub fn print_error(err: &SkmError, verbose: bool) {
    if verbose {
        print_error_verbose(err);
        return;
    }
    eprintln!("error: {}", err.leaf());
}

fn print_error_verbose(err: &SkmError) {
    match err {
        SkmError::WithContext { operation, source } => {
            eprintln!("error: {operation}");
            eprint!("  caused by: ");
            print_error_verbose_inner(source);
        }
        other => eprintln!("error: {other}"),
    }
}

fn print_error_verbose_inner(err: &SkmError) {
    match err {
        SkmError::WithContext { operation, source } => {
            eprintln!("{operation}");
            eprint!("  caused by: ");
            print_error_verbose_inner(source);
        }
        other => eprintln!("{other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_and_resolve_conflict_are_exit_two() {
        assert_eq!(SkmError::Usage("x".into()).exit_code(), 2);
        assert_eq!(SkmError::ResolveConflict("tdd".into()).exit_code(), 2);
        assert_eq!(SkmError::DuplicateSkillId("docx".into()).exit_code(), 2);
        assert_eq!(SkmError::InvalidSkillId("Bad".into()).exit_code(), 2);
        assert_eq!(SkmError::RefuseNonInteractiveRm.exit_code(), 2);
        assert_eq!(SkmError::RefuseNonInteractiveDestroy.exit_code(), 2);
    }

    #[test]
    fn wrapped_errors_use_source_exit_code() {
        let err = SkmError::ResolveConflict("tdd".into()).op("syncing skills");
        assert_eq!(err.exit_code(), 2);
        assert!(matches!(err.leaf(), SkmError::ResolveConflict(_)));
    }

    #[test]
    fn domain_failures_are_exit_one() {
        assert_eq!(SkmError::EmptyProfile.exit_code(), 1);
        assert_eq!(SkmError::StoreNotInitialized.exit_code(), 1);
        assert_eq!(SkmError::HomeNotFound.exit_code(), 1);
    }
}
