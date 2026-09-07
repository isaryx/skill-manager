pub mod discover;
pub mod git;
pub mod link;
pub mod meta;
pub mod paths;
pub mod pin;
pub mod register;
pub mod registry;
pub mod skills_sh;
pub mod unregister;
pub mod update;
pub mod url;

pub use paths::checkout_path;
pub use pin::{clear_repo_pin, set_repo_pin, show_repo_pin};
pub use register::register_repo;
pub use registry::{list_repos, read_repo};
pub use unregister::unregister_repo;
pub use meta::{
    record_all_remote_skill_syncs, record_remote_skill_sync, remove_remote_skill_meta,
    RemoteSyncRecord,
};
pub use skills_sh::fetch_leaderboard;
pub use update::{pull_remotes, skill_count_for_repo, PullOptions};
