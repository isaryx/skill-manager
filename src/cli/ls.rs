use crate::color::color_stdout;

use dialoguer::console::style;

use crate::cli::output::{write_json, LsJson};
use crate::error::SkmError;
use crate::store::profiles::list_profiles;
use crate::store::StorePaths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsFilter {
    All,
    Skills,
    Profiles,
}

pub fn run(store: &StorePaths, filter: LsFilter, json: bool) -> Result<(), SkmError> {
    match filter {
        LsFilter::Skills => list_skills(store, json),
        LsFilter::Profiles => list_profiles_lines(store, json),
        LsFilter::All => list_all(store, json),
    }
}

fn list_skills(store: &StorePaths, json: bool) -> Result<(), SkmError> {
    store
        .ensure_initialized()
        .map_err(|e| e.op("skill store is not initialized"))?;
    let ids = crate::store::list_pool_ids(store).map_err(|e| e.op("listing library skills"))?;

    if json {
        let payload = LsJson {
            skills: Some(ids),
            profiles: None,
        };
        write_json(&payload).map_err(|e| SkmError::Usage(format!("failed to encode JSON: {e}")))?;
        return Ok(());
    }

    for id in ids {
        println!("{id}");
    }
    Ok(())
}

fn list_profiles_lines(store: &StorePaths, json: bool) -> Result<(), SkmError> {
    let profiles = list_profiles(store).map_err(|e| e.op("listing profiles"))?;

    if json {
        let payload = LsJson {
            skills: None,
            profiles: Some(profiles),
        };
        write_json(&payload).map_err(|e| SkmError::Usage(format!("failed to encode JSON: {e}")))?;
        return Ok(());
    }

    for name in profiles {
        println!("{name}");
    }
    Ok(())
}

fn list_all(store: &StorePaths, json: bool) -> Result<(), SkmError> {
    store
        .ensure_initialized()
        .map_err(|e| e.op("skill store is not initialized"))?;

    let profiles = list_profiles(store).map_err(|e| e.op("listing profiles"))?;
    let skills = crate::store::list_pool_ids(store).map_err(|e| e.op("listing library skills"))?;

    if json {
        let payload = LsJson {
            skills: Some(skills),
            profiles: Some(profiles),
        };
        write_json(&payload).map_err(|e| SkmError::Usage(format!("failed to encode JSON: {e}")))?;
        return Ok(());
    }

    let mut wrote_section = false;

    if !profiles.is_empty() {
        print_section_header("Profiles");
        for name in profiles {
            println!("profile/{name}");
        }
        wrote_section = true;
    }

    if !skills.is_empty() {
        if wrote_section {
            println!();
        }
        print_section_header("Skills");
        for id in skills {
            println!("skill/{id}");
        }
    }

    Ok(())
}

fn print_section_header(title: &str) {
    if color_stdout() {
        println!("{}", style(title).dim());
    } else {
        println!("{title}");
    }
}
