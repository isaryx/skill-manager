use std::path::Path;

use crate::error::SkmError;
use crate::progress;
use crate::store::pool::{add_skill, add_skill_tree, TransferMode};
use crate::store::StorePaths;
use crate::util::skill_spec::check_skill_specs_for_import;
use crate::util::{discover_all_skill_dirs, is_skill_dir, is_skill_tree};

pub fn run_import(
    store: &StorePaths,
    dir: &Path,
    copy: bool,
    move_: bool,
    as_name: Option<String>,
    repo: Option<String>,
    strict: bool,
) -> Result<(), SkmError> {
    if !copy && !move_ {
        return Err(SkmError::Usage(
            "choose --copy (keep original) or --move (remove original)".to_string(),
        ));
    }
    if copy && move_ {
        return Err(SkmError::Usage(
            "--copy and --move are mutually exclusive".to_string(),
        ));
    }

    let mode = if copy {
        TransferMode::Copy
    } else {
        TransferMode::Move
    };
    let mode_label = if copy { "copying" } else { "moving" };

    if is_skill_dir(dir) && !is_skill_tree(dir) {
        check_skill_specs_for_import(&[dir.to_path_buf()], strict, as_name.as_deref())?;
        progress::step(format!("{mode_label} skill from {}", dir.display()));
        let name = add_skill(store, dir, mode, as_name.as_deref(), repo.as_deref())
            .map_err(|e| e.op(format!("importing skill from {}", dir.display())))?;
        progress::added(&name);
        return Ok(());
    }

    if is_skill_tree(dir) {
        let skill_dirs = discover_all_skill_dirs(dir)?;
        check_skill_specs_for_import(&skill_dirs, strict, as_name.as_deref())?;
        progress::step(format!("{mode_label} skill tree from {}", dir.display()));
        let skill_ids = add_skill_tree(store, dir, mode, as_name.as_deref(), repo.as_deref())
            .map_err(|e| e.op(format!("importing skills from {}", dir.display())))?;
        for id in &skill_ids {
            progress::added(id);
        }
        for id in skill_ids {
            println!("{id}");
        }
        return Ok(());
    }

    Err(SkmError::NotASkillDir(dir.to_path_buf()))
}
