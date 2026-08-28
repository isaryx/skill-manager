use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SkmError {
    #[error("store not initialized; run `skm init`")]
    StoreNotInitialized,

    #[error("config file already exists: {0}")]
    SetupExists(PathBuf),

    #[error("config file not found: {0}")]
    SetupNotFound(PathBuf),

    #[error("application config not found: {0}")]
    AppConfigNotFound(PathBuf),

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

    #[error("profile not found: {0}")]
    ProfileNotFound(String),

    #[error("profile is empty; add skills with `skm profile setup`")]
    EmptyProfile,

    #[error("no active profile; run `skm use-profile <profile>`")]
    NoActiveProfile,

    #[error("cannot remove active profile `{0}`; switch profiles first")]
    ActiveProfileRemoval(String),

    #[error("duplicate skill id in profile: {0}")]
    DuplicateSkillId(String),

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
            | SkmError::RefuseNonInteractiveRm => 2,
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
