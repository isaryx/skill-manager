use std::env;

use crate::error::SkmError;
use crate::progress;
use crate::store::StorePaths;
use crate::sync::{reconcile, ReconcileOptions};

pub fn run_sync(
    store: &StorePaths,
    force_user: bool,
    options: ReconcileOptions,
) -> Result<(), SkmError> {
    let cwd = env::current_dir()?;
    if options.dry_run {
        progress::step("(dry-run) syncing skills");
    } else {
        progress::step("syncing skills");
    }
    reconcile(store, &cwd, force_user, options).map_err(|e| e.op("syncing skills"))?;
    Ok(())
}
