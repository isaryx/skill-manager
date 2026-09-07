use crate::error::SkmError;
use crate::progress;
use crate::store::remote::{pull_remotes, PullOptions};
use crate::store::StorePaths;

pub fn run_update(store: &StorePaths, name: Option<&str>, dry_run: bool) -> Result<(), SkmError> {
    if dry_run {
        progress::step("(dry-run) updating remote repositories");
    } else {
        progress::step("updating remote repositories");
    }

    if let Some(name) = name {
        crate::util::validate_store_entry_name(name)?;
    }

    pull_remotes(
        store,
        PullOptions {
            dry_run,
            repo_filter: name.map(str::to_string),
        },
    )
    .map_err(|e| e.op("updating remote repositories"))?;

    Ok(())
}
