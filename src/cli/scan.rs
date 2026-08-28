use crate::db::{list_skills, open_index, refresh_store_index};
use crate::error::SkmError;
use crate::progress;
use crate::store::{ensure_store_subdirs, StorePaths};

pub fn run_scan(store: &StorePaths) -> Result<(), SkmError> {
    store.ensure_initialized()?;
    ensure_store_subdirs(store)?;
    progress::step(format!(
        "scanning skill store at {}",
        progress::display_path(store.root())
    ));
    refresh_store_index(store).map_err(|e| e.op("scanning skill store"))?;

    let conn = open_index(store)?;
    let skills = list_skills(&conn)?;
    progress::step(format!("indexed {} skill(s)", skills.len()));

    Ok(())
}
