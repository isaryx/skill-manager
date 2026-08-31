use std::env;
use std::io::{self, IsTerminal};

use crate::color::color_stdout;
use crate::error::SkmError;
use crate::progress;
use crate::setup::select_setup;
use crate::store::extends::{
    build_tree, extend_candidates, flatten_profile, flatten_with_extends, profiles_extending,
    render_tree,
};
use crate::store::profiles::{
    create_profile, ensure_profile, ensure_profile_not_active, interactive_setup, load_profile,
    remove_profile, set_profile_extends, set_profile_skills,
};
use crate::store::skills::read_disabled_ids;
use crate::store::StorePaths;
use crate::tui::{MultiSelect, MultiSelectItem};
use crate::util::validate_profile_name;

pub fn run_profile_setup(store: &StorePaths, name: &str) -> Result<(), SkmError> {
    validate_profile_name(name)?;
    if !io::stdin().is_terminal() {
        return Err(SkmError::NotATty);
    }

    let existed = store.profile_file(name).is_file();
    let existing: Vec<String> = if existed {
        load_profile(store, name)
            .map_err(|e| e.op(format!("loading profile `{name}`")))?
            .skill
            .into_iter()
            .map(|s| s.id)
            .collect()
    } else {
        Vec::new()
    };

    let skill_ids = interactive_setup(store, name, &existing)
        .map_err(|e| e.op(format!("setting up profile `{name}`")))?;

    if !existed {
        create_profile(store, name, &skill_ids)
            .map_err(|e| e.op(format!("creating profile `{name}`")))?;
    } else {
        set_profile_skills(store, name, &skill_ids)
            .map_err(|e| e.op(format!("writing profile `{name}`")))?;
    }

    let verb = if existed { "updated" } else { "created" };
    progress::step(format!(
        "{verb} profile `{name}` with {} skill(s)",
        skill_ids.len()
    ));
    Ok(())
}

/// Pick the profiles `name` extends. Creates `name` if it does not exist yet, matching `setup`.
pub fn run_profile_extend(store: &StorePaths, name: &str) -> Result<(), SkmError> {
    validate_profile_name(name)?;
    if !io::stdin().is_terminal() {
        return Err(SkmError::NotATty);
    }

    // Create only after a confirmed selection, as `setup` does — bailing out below should not
    // leave a profile behind.
    let existed = store.profile_file(name).is_file();
    let selected: Vec<String> = if existed {
        load_profile(store, name)
            .map_err(|e| e.op(format!("loading profile `{name}`")))?
            .extends
    } else {
        Vec::new()
    };

    // Anything that already reaches `name` is left out rather than offered and then refused:
    // a checkbox that cannot be ticked is a worse affordance than an absent one.
    let candidates = extend_candidates(store, name)
        .map_err(|e| e.op(format!("listing extend candidates for `{name}`")))?;
    if candidates.is_empty() {
        return Err(SkmError::NoExtendCandidates);
    }

    let selected_set: std::collections::HashSet<&str> =
        selected.iter().map(String::as_str).collect();
    let items = candidates.iter().map(|candidate| {
        MultiSelectItem::new(candidate).selected(selected_set.contains(candidate.as_str()))
    });

    let chosen = MultiSelect::new(format!("Profiles `{name}` inherits skills from"))
        .items(items)
        .interact()
        .map_err(|e| e.op(format!("choosing profiles extended by `{name}`")))?;

    // Validate the graph the selection implies *before* writing any of it. Writing first would
    // persist a selection that is then rejected, leaving the profile broken on disk and failing
    // every later `use-profile` until the user re-ran this command and guessed what to deselect.
    let flat = flatten_with_extends(store, name, &chosen)
        .map_err(|e| e.op(format!("checking profiles extended by `{name}`")))?;

    ensure_profile(store, name).map_err(|e| e.op(format!("creating profile `{name}`")))?;
    set_profile_extends(store, name, &chosen)
        .map_err(|e| e.op(format!("writing profile `{name}`")))?;

    let verb = if existed { "updated" } else { "created" };
    if chosen.is_empty() {
        progress::step(format!("{verb} profile `{name}`; extends nothing"));
    } else {
        progress::step(format!(
            "{verb} profile `{name}`; extends {} ({} skill(s) total)",
            chosen.join(", "),
            flat.len()
        ));
    }
    Ok(())
}

pub fn run_profile_ls(store: &StorePaths) -> Result<(), SkmError> {
    crate::cli::ls::run(store, crate::cli::ls::LsFilter::Profiles, false)
}

pub fn run_profile_show(
    store: &StorePaths,
    name: &str,
    force_user: bool,
    tree: bool,
) -> Result<(), SkmError> {
    validate_profile_name(name)?;

    let cwd = env::current_dir()?;
    if let Ok(setup) = select_setup(&cwd, force_user) {
        if setup.setup.profile.active.as_deref() == Some(name) {
            eprintln!("(active)");
        }
    }

    let disabled = read_disabled_ids(store)?;
    if tree {
        return show_tree(store, name, &disabled);
    }

    let profile =
        load_profile(store, name).map_err(|e| e.op(format!("loading profile `{name}`")))?;
    if !profile.extends.is_empty() {
        eprintln!("(extends {})", profile.extends.join(", "));
    }

    let flat =
        flatten_profile(store, name).map_err(|e| e.op(format!("flattening profile `{name}`")))?;

    for skill in &flat {
        let mut notes = Vec::new();
        if let Some(origin) = &skill.from {
            notes.push(format!("from {origin}"));
        }
        if disabled.contains(&skill.id) {
            notes.push("disabled".to_string());
        }
        if notes.is_empty() {
            println!("{}", skill.id);
        } else {
            println!("{} ({})", skill.id, notes.join(", "));
        }
    }
    Ok(())
}

/// Print the extend graph. The `(extends …)` note is skipped here — the tree already shows it.
///
/// A broken graph is rendered rather than refused, then reported: the tree is exactly what you
/// want to look at when `use-profile` rejects a profile. Failing afterwards keeps the exit code
/// identical to the flat listing's for the same graph.
fn show_tree(
    store: &StorePaths,
    name: &str,
    disabled: &std::collections::HashSet<String>,
) -> Result<(), SkmError> {
    let tree = build_tree(store, name, disabled);

    for line in render_tree(&tree.root, color_stdout()) {
        println!("{line}");
    }
    println!();
    let mut summary = format!(
        "{} skill{} resolved",
        tree.resolved,
        if tree.resolved == 1 { "" } else { "s" }
    );
    // Disabled skills are part of the profile but never wired, so the two numbers differ.
    if tree.disabled > 0 {
        summary.push_str(&format!(", {} disabled and not wired", tree.disabled));
    }
    println!("{summary}");

    match tree.error {
        Some(err) => Err(err.op(format!("flattening profile `{name}`"))),
        None => Ok(()),
    }
}

pub fn run_profile_rm(store: &StorePaths, name: &str) -> Result<(), SkmError> {
    validate_profile_name(name)?;
    let cwd = env::current_dir()?;
    ensure_profile_not_active(&cwd, name)
        .map_err(|e| e.op(format!("checking active profile for `{name}`")))?;

    // Mirrors the active-profile guard: refuse rather than leave dangling `extends` behind.
    let extenders = profiles_extending(store, name)
        .map_err(|e| e.op(format!("checking profiles extending `{name}`")))?;
    if !extenders.is_empty() {
        return Err(SkmError::ExtendedProfileRemoval {
            profile: name.to_string(),
            extenders: extenders.join(", "),
        });
    }

    remove_profile(store, name).map_err(|e| e.op(format!("removing profile `{name}`")))?;
    progress::step(format!("removed profile `{name}`"));
    Ok(())
}
