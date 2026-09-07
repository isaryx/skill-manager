pub mod discover;
pub mod git;
pub mod link;
pub mod paths;
pub mod register;
pub mod registry;
pub mod update;
pub mod url;

pub use paths::checkout_path;
pub use register::register_repo;
pub use registry::{list_repos, read_repo};
pub use update::{pull_remotes, skill_count_for_repo, PullOptions};
