//! Profile inheritance: flattening `extends` into a single skill list.
//!
//! `extends` is a live reference, not a copy — the graph is walked every time a profile is
//! resolved for placement, so editing a base profile immediately affects everything that
//! extends it. Profile files are hand-editable, so cycles and over-deep chains are rejected
//! here rather than trusted to whatever wrote them.

use std::collections::HashSet;

use dialoguer::console::style;

use crate::config::{ProfileFile, ProfileSkillEntry};
use crate::error::SkmError;
use crate::store::profiles::{list_profiles, load_profile};
use crate::store::StorePaths;

/// Longest extend chain we accept, counted in hops from the starting profile.
///
/// Cycle detection, not this limit, is what makes flattening terminate. The limit exists so a
/// pathological hand-written chain fails with a clear message instead of quietly producing a
/// profile nobody can reason about. Realistic hierarchies (`base → org → team → personal`) are
/// around three hops, so this never fires in practice.
///
/// Changing this means updating the places that spell the number out: the `profile extend` help
/// in `cli/mod.rs`, and the inheritance section of `docs/SPEC.md`.
pub const MAX_EXTEND_DEPTH: usize = 8;

/// One skill in a flattened profile, with where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatSkill {
    pub id: String,
    /// `None` when the profile declares the skill itself, otherwise the profile it came from.
    pub from: Option<String>,
}

/// Flatten `name` and everything it extends into one deduplicated skill list.
///
/// Order is the profile's own skills first, then inherited ones depth-first in `extends` order.
/// The first occurrence of an ID wins, so a skill declared directly is attributed to the profile
/// itself even when a base profile also lists it.
pub fn flatten_profile(store: &StorePaths, name: &str) -> Result<Vec<FlatSkill>, SkmError> {
    flatten(store, name, None)
}

/// Flatten as if `name` extended exactly `extends`, without writing anything.
///
/// Lets `profile extend` reject a cycle or an over-deep chain *before* persisting the selection.
/// `name` need not exist yet; a missing root contributes no skills of its own.
pub fn flatten_with_extends(
    store: &StorePaths,
    name: &str,
    extends: &[String],
) -> Result<Vec<FlatSkill>, SkmError> {
    flatten(store, name, Some(extends))
}

fn flatten(
    store: &StorePaths,
    name: &str,
    root_extends: Option<&[String]>,
) -> Result<Vec<FlatSkill>, SkmError> {
    let mut walk = Walk {
        store,
        root: name,
        root_extends,
        path: Vec::new(),
        expanded: HashSet::new(),
        seen_skills: HashSet::new(),
        flat: Vec::new(),
    };
    walk.visit(name, None)?;
    Ok(walk.flat)
}

/// Depth-first walk of the extend graph.
///
/// `path` holds the chain currently being expanded and is what detects cycles: an edge back to
/// anything already on it closes a loop. `expanded` is only memoization, so a diamond is walked
/// once instead of once per route.
///
/// Correctness rests on this being *recursive*: `visit` does not return until the whole subtree
/// below a profile is expanded, so any route that re-enters that profile meanwhile still finds it
/// on `path`. An iterative walk with one shared "visited" set does not have that property — it
/// can mark a profile before its subtree is done and then skip it on a branch where the cycle is
/// visible. Recursion is safe in turn only because [`MAX_EXTEND_DEPTH`] caps the depth, which is
/// what makes that limit load-bearing rather than cosmetic.
struct Walk<'a> {
    store: &'a StorePaths,
    root: &'a str,
    /// When set, the root's `extends` on disk is ignored in favour of this hypothetical list.
    root_extends: Option<&'a [String]>,
    path: Vec<String>,
    expanded: HashSet<String>,
    seen_skills: HashSet<String>,
    flat: Vec<FlatSkill>,
}

impl Walk<'_> {
    fn visit(&mut self, current: &str, parent: Option<&str>) -> Result<(), SkmError> {
        if self.path.iter().any(|seen| seen == current) {
            let mut cycle = self.path.clone();
            cycle.push(current.to_string());
            return Err(SkmError::ExtendCycle(cycle.join(" → ")));
        }
        // Depth is judged before the `expanded` short-circuit below. Checking it after would
        // make the verdict depend on which branch `extends` happens to list first: a profile
        // already expanded via a short route would silently skip the check.
        if self.path.len() > MAX_EXTEND_DEPTH {
            let mut chain = self.path.clone();
            chain.push(current.to_string());
            return Err(SkmError::ExtendTooDeep {
                limit: MAX_EXTEND_DEPTH,
                chain: chain.join(" → "),
            });
        }
        // A diamond — two profiles extending a common base — is legal and common.
        if self.expanded.contains(current) {
            return Ok(());
        }

        let loaded = load_profile(self.store, current);
        let hypothetical_root =
            current == self.root && self.root_extends.is_some() && is_missing(&loaded);
        let profile = if hypothetical_root {
            // Validating a selection for a profile that does not exist yet: it has no skills of
            // its own, and its `extends` comes from the override below.
            ProfileFile::default()
        } else {
            loaded.map_err(|err| {
                // Tell "you extended something that is not there" apart from a broken store.
                match (parent, err.leaf()) {
                    (Some(parent), SkmError::ProfileNotFound(_)) => SkmError::ExtendNotFound {
                        profile: parent.to_string(),
                        missing: current.to_string(),
                    },
                    _ => err,
                }
            })?
        };

        let origin = (current != self.root).then(|| current.to_string());
        for entry in &profile.skill {
            if self.seen_skills.insert(entry.id.clone()) {
                self.flat.push(FlatSkill {
                    id: entry.id.clone(),
                    from: origin.clone(),
                });
            }
        }

        // Cloned so the recursive call below can borrow `self` mutably.
        let targets: Vec<String> = match self.root_extends {
            Some(override_extends) if current == self.root => override_extends.to_vec(),
            _ => profile.extends.clone(),
        };

        self.path.push(current.to_string());
        for target in &targets {
            self.visit(target, Some(current))?;
        }
        self.path.pop();
        self.expanded.insert(current.to_string());
        Ok(())
    }
}

fn is_missing(loaded: &Result<ProfileFile, SkmError>) -> bool {
    matches!(loaded, Err(err) if matches!(err.leaf(), SkmError::ProfileNotFound(_)))
}

/// Flattened skill IDs only, in the same order as [`flatten_profile`].
pub fn flatten_skill_ids(store: &StorePaths, name: &str) -> Result<Vec<String>, SkmError> {
    Ok(flatten_profile(store, name)?
        .into_iter()
        .map(|skill| skill.id)
        .collect())
}

/// `name` with its extend graph already flattened into the skill list.
///
/// Lets `resolver::resolve` stay pure and I/O-free: the graph walk happens here, and the
/// resolver still receives a plain [`ProfileFile`]. The returned value has an empty `extends`,
/// since it is already resolved.
pub fn load_merged_flattened_profile(
    store: &StorePaths,
    names: &[impl AsRef<str>],
) -> Result<ProfileFile, SkmError> {
    let mut seen = std::collections::HashSet::new();
    let mut skill = Vec::new();
    for name in names {
        for id in flatten_skill_ids(store, name.as_ref())? {
            if seen.insert(id.clone()) {
                skill.push(ProfileSkillEntry { id });
            }
        }
    }
    Ok(ProfileFile {
        extends: Vec::new(),
        skill,
    })
}

pub fn load_flattened_profile(store: &StorePaths, name: &str) -> Result<ProfileFile, SkmError> {
    load_merged_flattened_profile(store, &[name])
}

/// What a [`TreeNode`] represents. Stated rather than inferred: an empty profile is a childless
/// leaf, so "has no children" does not mean "is a skill".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Profile,
    Skill,
}

/// One line of a rendered extend tree: a profile, or a skill under the profile declaring it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    pub kind: NodeKind,
    /// Profile name or skill ID.
    pub label: String,
    /// Dim suffixes, joined with `, ` when rendered. Deliberately short fixed labels — `*`,
    /// `disabled`, `not found`, `unreadable`, `cycle`, `too deep` — never error text, which can
    /// be multi-line and would break the tree's shape.
    pub notes: Vec<&'static str>,
    /// True for a node whose graph is broken, so rendering can colour it as a problem.
    pub broken: bool,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    fn profile(label: impl Into<String>) -> Self {
        Self::new(NodeKind::Profile, label)
    }

    fn skill(label: impl Into<String>) -> Self {
        Self::new(NodeKind::Skill, label)
    }

    fn new(kind: NodeKind, label: impl Into<String>) -> Self {
        Self {
            kind,
            label: label.into(),
            notes: Vec::new(),
            broken: false,
            children: Vec::new(),
        }
    }

    fn note(mut self, note: &'static str) -> Self {
        self.notes.push(note);
        self
    }

    fn broken(mut self, note: &'static str) -> Self {
        self.notes.push(note);
        self.broken = true;
        self
    }
}

/// Build the extend tree for `name`, tolerating a broken graph.
///
/// Returns the tree plus the first problem found, if any. Unlike [`flatten_profile`] this never
/// gives up early: a cycle, missing profile or over-deep chain is marked in place and the walk
/// continues through the siblings, because seeing the whole graph is the point of the view. The
/// returned error lets the caller exit with the same code plain `profile show` would.
///
/// `*` marks a node already accounted for above — a profile subtree rendered elsewhere, or a
/// skill an earlier profile already contributed. Those are not counted twice, so `Tree::resolved`
/// matches the line count of the flat `profile show` listing. It is *not* the number of symlinks
/// `use-profiles` creates: disabled skills resolve but are never wired.
pub fn build_tree(store: &StorePaths, name: &str, disabled: &HashSet<String>) -> Tree {
    let mut builder = TreeBuilder {
        store,
        disabled,
        path: Vec::new(),
        expanded: HashSet::new(),
        seen_skills: HashSet::new(),
        resolved: 0,
        disabled_count: 0,
        first_error: None,
    };
    let root = builder.visit(name, None);
    Tree {
        root,
        resolved: builder.resolved,
        disabled: builder.disabled_count,
        error: builder.first_error,
    }
}

/// A built extend tree.
#[derive(Debug)]
pub struct Tree {
    pub root: TreeNode,
    /// Distinct skills the profile resolves to — every node not marked `*`. Matches the line
    /// count of the flat `profile show` listing.
    pub resolved: usize,
    /// How many of `resolved` are disabled, and so resolve but are not wired.
    pub disabled: usize,
    /// The first problem found, if the graph is broken. Lets the caller fail with the same exit
    /// code plain `profile show` would, after printing the tree.
    pub error: Option<SkmError>,
}

struct TreeBuilder<'a> {
    store: &'a StorePaths,
    disabled: &'a HashSet<String>,
    path: Vec<String>,
    expanded: HashSet<String>,
    seen_skills: HashSet<String>,
    resolved: usize,
    disabled_count: usize,
    first_error: Option<SkmError>,
}

impl TreeBuilder<'_> {
    fn record(&mut self, err: SkmError) {
        if self.first_error.is_none() {
            self.first_error = Some(err);
        }
    }

    fn visit(&mut self, current: &str, parent: Option<&str>) -> TreeNode {
        if self.path.iter().any(|seen| seen == current) {
            let mut chain = self.path.clone();
            chain.push(current.to_string());
            self.record(SkmError::ExtendCycle(chain.join(" → ")));
            return TreeNode::profile(current).broken("cycle");
        }
        if self.path.len() > MAX_EXTEND_DEPTH {
            let mut chain = self.path.clone();
            chain.push(current.to_string());
            self.record(SkmError::ExtendTooDeep {
                limit: MAX_EXTEND_DEPTH,
                chain: chain.join(" → "),
            });
            return TreeNode::profile(current).broken("too deep");
        }
        if self.expanded.contains(current) {
            return TreeNode::profile(current).note("*");
        }

        let profile = match load_profile(self.store, current) {
            Ok(profile) => profile,
            Err(err) => {
                let broken = match (parent, err.leaf()) {
                    (Some(parent), SkmError::ProfileNotFound(_)) => SkmError::ExtendNotFound {
                        profile: parent.to_string(),
                        missing: current.to_string(),
                    },
                    _ => err,
                };
                // Notes are a closed vocabulary of short labels, never the error text: a TOML
                // parse error is multi-line with caret art, and embedding it would break the
                // tree's shape. The full message is carried by `Tree::error` and printed after.
                let note = match broken.leaf() {
                    // `ProfileNotFound` for the root, `ExtendNotFound` for anything below it.
                    SkmError::ProfileNotFound(_) | SkmError::ExtendNotFound { .. } => "not found",
                    _ => "unreadable",
                };
                self.record(broken);
                return TreeNode::profile(current).broken(note);
            }
        };

        let mut node = TreeNode::profile(current);
        for entry in &profile.skill {
            let mut skill = TreeNode::skill(&entry.id);
            if self.seen_skills.insert(entry.id.clone()) {
                self.resolved += 1;
                if self.disabled.contains(&entry.id) {
                    self.disabled_count += 1;
                    skill = skill.note("disabled");
                }
            } else {
                // Declared here too, but an earlier profile already contributed it.
                skill = skill.note("*");
            }
            node.children.push(skill);
        }

        self.path.push(current.to_string());
        let targets = profile.extends.clone();
        for target in &targets {
            let child = self.visit(target, Some(current));
            node.children.push(child);
        }
        self.path.pop();
        self.expanded.insert(current.to_string());
        node
    }
}

/// Render a tree as one string per line, `cargo tree` style.
pub fn render_tree(root: &TreeNode, color: bool) -> Vec<String> {
    let mut lines = vec![styled_label(root, color, true)];
    push_children(root, "", &mut lines, color);
    lines
}

fn push_children(node: &TreeNode, prefix: &str, lines: &mut Vec<String>, color: bool) {
    let last_index = node.children.len().saturating_sub(1);
    for (index, child) in node.children.iter().enumerate() {
        let is_last = index == last_index;
        let connector = if is_last { "└── " } else { "├── " };
        lines.push(format!(
            "{prefix}{connector}{}",
            styled_label(child, color, false)
        ));
        let child_prefix = format!("{prefix}{}", if is_last { "    " } else { "│   " });
        push_children(child, &child_prefix, lines, color);
    }
}

fn styled_label(node: &TreeNode, color: bool, root: bool) -> String {
    let label = if color && root {
        style(&node.label).bold().to_string()
    } else {
        node.label.clone()
    };
    if node.notes.is_empty() {
        return label;
    }
    let notes = format!("({})", node.notes.join(", "));
    let notes = if !color {
        notes
    } else if node.broken {
        style(&notes).red().to_string()
    } else {
        style(&notes).dim().to_string()
    };
    format!("{label} {notes}")
}

/// Profiles that list `name` in their own `extends` (direct extenders only).
pub fn profiles_extending(store: &StorePaths, name: &str) -> Result<Vec<String>, SkmError> {
    let mut extenders = Vec::new();
    for candidate in list_profiles(store)? {
        if candidate == name {
            continue;
        }
        if load_profile(store, &candidate)?
            .extends
            .iter()
            .any(|target| target == name)
        {
            extenders.push(candidate);
        }
    }
    Ok(extenders)
}

/// Profiles `name` may extend: every other profile that does not already reach `name`.
///
/// Extending one of those would close a cycle, so they are not offered as choices at all.
pub fn extend_candidates(store: &StorePaths, name: &str) -> Result<Vec<String>, SkmError> {
    let mut candidates = Vec::new();
    for candidate in list_profiles(store)? {
        if candidate == name {
            continue;
        }
        // `flatten_profile` walks the candidate's own graph; if `name` is reachable from it,
        // then `name → candidate` would complete a loop.
        if !reaches(store, &candidate, name)? {
            candidates.push(candidate);
        }
    }
    Ok(candidates)
}

/// Whether `target` is `from` or reachable from it through `extends`.
fn reaches(store: &StorePaths, from: &str, target: &str) -> Result<bool, SkmError> {
    let mut visited = HashSet::new();
    let mut stack = vec![from.to_string()];
    while let Some(current) = stack.pop() {
        if current == target {
            return Ok(true);
        }
        if !visited.insert(current.clone()) {
            continue;
        }
        // A broken or unreadable link cannot reach anything; leave reporting to `flatten_profile`.
        let Ok(profile) = load_profile(store, &current) else {
            continue;
        };
        stack.extend(profile.extends.iter().cloned());
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{init_store_layout, StorePaths};
    use std::fs;
    use tempfile::TempDir;

    /// Store with the given `(profile, extends, skills)` triples written straight to disk.
    /// `(profile name, extends, own skills)`.
    type Shape<'a> = (&'a str, &'a [&'a str], &'a [&'a str]);

    fn store_with(profiles: &[Shape]) -> (TempDir, StorePaths) {
        let tmp = TempDir::new().unwrap();
        let store = StorePaths::new(tmp.path().to_path_buf());
        init_store_layout(&store).unwrap();

        for (name, extends, skills) in profiles {
            let mut toml = String::new();
            if !extends.is_empty() {
                let list: Vec<String> = extends.iter().map(|e| format!("\"{e}\"")).collect();
                toml.push_str(&format!("extends = [{}]\n", list.join(", ")));
            }
            for skill in *skills {
                toml.push_str(&format!("\n[[skill]]\nid = \"{skill}\"\n"));
            }
            fs::write(store.profile_file(name), toml).unwrap();
        }
        (tmp, store)
    }

    fn ids(store: &StorePaths, name: &str) -> Vec<String> {
        flatten_skill_ids(store, name).unwrap()
    }

    // ---- flattening ---------------------------------------------------------------

    #[test]
    fn own_skills_come_first_then_inherited_depth_first() {
        let (_tmp, store) = store_with(&[
            ("app", &["web", "ops"], &["own"]),
            ("web", &["base"], &["html"]),
            ("ops", &[], &["tf"]),
            ("base", &[], &["git"]),
        ]);

        // app's own, then web, then web's base, then ops — depth-first in `extends` order.
        assert_eq!(ids(&store, "app"), vec!["own", "html", "git", "tf"]);
    }

    #[test]
    fn inherited_skills_are_attributed_to_their_profile() {
        let (_tmp, store) = store_with(&[("app", &["base"], &["own"]), ("base", &[], &["git"])]);

        let flat = flatten_profile(&store, "app").unwrap();
        assert_eq!(
            flat[0],
            FlatSkill {
                id: "own".into(),
                from: None
            }
        );
        assert_eq!(
            flat[1],
            FlatSkill {
                id: "git".into(),
                from: Some("base".into())
            }
        );
    }

    #[test]
    fn a_directly_declared_skill_is_not_attributed_to_a_base() {
        let (_tmp, store) = store_with(&[("app", &["base"], &["git"]), ("base", &[], &["git"])]);

        let flat = flatten_profile(&store, "app").unwrap();
        assert_eq!(flat.len(), 1, "{flat:?}");
        assert_eq!(flat[0].from, None, "direct declaration wins attribution");
    }

    #[test]
    fn a_diamond_yields_each_skill_once() {
        let (_tmp, store) = store_with(&[
            ("app", &["left", "right"], &[]),
            ("left", &["base"], &["l"]),
            ("right", &["base"], &["r"]),
            ("base", &[], &["shared"]),
        ]);

        assert_eq!(ids(&store, "app"), vec!["l", "shared", "r"]);
    }

    #[test]
    fn a_profile_with_no_skills_of_its_own_is_fine() {
        let (_tmp, store) = store_with(&[
            ("meta", &["a", "b"], &[]),
            ("a", &[], &["one"]),
            ("b", &[], &["two"]),
        ]);

        assert_eq!(ids(&store, "meta"), vec!["one", "two"]);
    }

    #[test]
    fn a_profile_without_extends_flattens_to_its_own_skills() {
        let (_tmp, store) = store_with(&[("solo", &[], &["a", "b"])]);
        assert_eq!(ids(&store, "solo"), vec!["a", "b"]);
    }

    // ---- cycles -------------------------------------------------------------------

    #[test]
    fn a_two_profile_cycle_is_rejected() {
        let (_tmp, store) = store_with(&[("a", &["b"], &[]), ("b", &["a"], &[])]);

        let err = flatten_profile(&store, "a").unwrap_err();
        assert!(
            matches!(&err, SkmError::ExtendCycle(chain) if chain == "a → b → a"),
            "{err:?}"
        );
    }

    #[test]
    fn a_cycle_not_involving_the_starting_profile_is_rejected() {
        let (_tmp, store) =
            store_with(&[("app", &["a"], &[]), ("a", &["b"], &[]), ("b", &["a"], &[])]);

        let err = flatten_profile(&store, "app").unwrap_err();
        assert!(
            matches!(&err, SkmError::ExtendCycle(chain) if chain == "app → a → b → a"),
            "{err:?}"
        );
    }

    /// A cycle in a second branch, reached after a first branch has already expanded a shared
    /// node — the shape where a walk that memoizes too eagerly can stop looking.
    #[test]
    fn a_cycle_reachable_only_through_a_later_branch_is_rejected() {
        let (_tmp, store) = store_with(&[
            ("root", &["first", "second"], &[]),
            ("first", &["shared"], &[]),
            ("shared", &[], &["s"]),
            ("second", &["x"], &[]),
            ("x", &["y"], &[]),
            ("y", &["x"], &[]),
        ]);

        let err = flatten_profile(&store, "root").unwrap_err();
        assert!(matches!(err, SkmError::ExtendCycle(_)), "{err:?}");
    }

    #[test]
    fn a_three_profile_cycle_is_rejected_from_every_entry_point() {
        let (_tmp, store) =
            store_with(&[("a", &["b"], &[]), ("b", &["c"], &[]), ("c", &["a"], &[])]);

        for entry in ["a", "b", "c"] {
            let err = flatten_profile(&store, entry).unwrap_err();
            assert!(matches!(err, SkmError::ExtendCycle(_)), "{entry}: {err:?}");
        }
    }

    #[test]
    fn a_self_extending_profile_is_rejected_when_read() {
        let (_tmp, store) = store_with(&[("a", &["a"], &[])]);
        let err = flatten_profile(&store, "a").unwrap_err();
        assert!(matches!(err.leaf(), SkmError::SelfExtend(_)), "{err:?}");
    }

    // ---- depth --------------------------------------------------------------------

    #[test]
    fn a_chain_at_the_depth_limit_is_accepted() {
        let names: Vec<String> = (0..=MAX_EXTEND_DEPTH).map(|i| format!("p{i}")).collect();
        let (_tmp, store) = store_with(&[]);
        for (i, name) in names.iter().enumerate() {
            let next = names.get(i + 1);
            let extends = next
                .map(|n| format!("extends = [\"{n}\"]\n"))
                .unwrap_or_default();
            fs::write(
                store.profile_file(name),
                format!("{extends}\n[[skill]]\nid = \"s{i}\"\n"),
            )
            .unwrap();
        }

        // p0 → p8 is MAX_EXTEND_DEPTH hops.
        let flat = ids(&store, "p0");
        assert_eq!(flat.len(), MAX_EXTEND_DEPTH + 1, "{flat:?}");
    }

    #[test]
    fn a_chain_past_the_depth_limit_is_rejected() {
        let names: Vec<String> = (0..=MAX_EXTEND_DEPTH + 1)
            .map(|i| format!("p{i}"))
            .collect();
        let (_tmp, store) = store_with(&[]);
        for (i, name) in names.iter().enumerate() {
            let next = names.get(i + 1);
            let extends = next
                .map(|n| format!("extends = [\"{n}\"]\n"))
                .unwrap_or_default();
            fs::write(store.profile_file(name), &extends).unwrap();
        }

        let err = flatten_profile(&store, "p0").unwrap_err();
        match err {
            SkmError::ExtendTooDeep { limit, chain } => {
                assert_eq!(limit, MAX_EXTEND_DEPTH);
                assert!(chain.starts_with("p0 → p1"), "{chain}");
            }
            other => panic!("{other:?}"),
        }
    }

    /// Whether a chain is too deep must not depend on the order `extends` happens to list.
    ///
    /// Both graphs here are identical: `root` reaches `deep` directly *and* through a nine-hop
    /// chain. Only the declaration order differs, so both must reach the same verdict.
    #[test]
    fn the_depth_verdict_does_not_depend_on_extends_order() {
        let over_limit = MAX_EXTEND_DEPTH + 1;

        let mut verdicts = Vec::new();
        for shortcut_first in [true, false] {
            let (_tmp, store) = store_with(&[]);

            // c1 → c2 → … → c{over_limit-1} → deep
            for i in 1..over_limit {
                let next = if i + 1 < over_limit {
                    format!("c{}", i + 1)
                } else {
                    "deep".to_string()
                };
                fs::write(
                    store.profile_file(&format!("c{i}")),
                    format!("extends = [\"{next}\"]\n"),
                )
                .unwrap();
            }
            fs::write(store.profile_file("deep"), "\n[[skill]]\nid = \"s\"\n").unwrap();

            let extends = if shortcut_first {
                "[\"deep\", \"c1\"]"
            } else {
                "[\"c1\", \"deep\"]"
            };
            fs::write(store.profile_file("root"), format!("extends = {extends}\n")).unwrap();

            verdicts.push(flatten_profile(&store, "root").is_ok());
        }

        assert_eq!(
            verdicts[0], verdicts[1],
            "shortcut-first gave {:?}, chain-first gave {:?}",
            verdicts[0], verdicts[1]
        );
    }

    /// `profile extend` validates before writing, so the check has to work on a hypothetical
    /// list and on a profile that does not exist yet.
    #[test]
    fn a_hypothetical_over_deep_selection_is_rejected_without_touching_disk() {
        let (_tmp, store) = store_with(&[]);
        // c1 → … → c{MAX+1}: choosing c1 from a new profile would be MAX+1 hops.
        for i in 1..=MAX_EXTEND_DEPTH + 1 {
            let extends = if i <= MAX_EXTEND_DEPTH {
                format!("extends = [\"c{}\"]\n", i + 1)
            } else {
                String::new()
            };
            fs::write(store.profile_file(&format!("c{i}")), extends).unwrap();
        }

        let err = flatten_with_extends(&store, "fresh", &["c1".to_string()]).unwrap_err();
        assert!(matches!(err, SkmError::ExtendTooDeep { .. }), "{err:?}");
        assert!(
            !store.profile_file("fresh").exists(),
            "validation must not create the profile"
        );
    }

    #[test]
    fn a_hypothetical_selection_within_the_limit_is_accepted_for_a_new_profile() {
        let (_tmp, store) = store_with(&[("base", &[], &["git"])]);

        let flat = flatten_with_extends(&store, "fresh", &["base".to_string()]).unwrap();
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].from.as_deref(), Some("base"));
        assert!(!store.profile_file("fresh").exists());
    }

    #[test]
    fn a_hypothetical_selection_ignores_the_extends_already_on_disk() {
        let (_tmp, store) = store_with(&[
            ("app", &["old"], &[]),
            ("old", &[], &["stale"]),
            ("new", &[], &["fresh"]),
        ]);

        let flat = flatten_with_extends(&store, "app", &["new".to_string()]).unwrap();
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].id, "fresh");
    }

    #[test]
    fn a_hypothetical_selection_that_closes_a_cycle_is_rejected() {
        let (_tmp, store) = store_with(&[("app", &[], &[]), ("mid", &["app"], &[])]);

        let err = flatten_with_extends(&store, "app", &["mid".to_string()]).unwrap_err();
        assert!(matches!(err, SkmError::ExtendCycle(_)), "{err:?}");
    }

    /// A chain that is both over-deep and cyclic reports one verdict, deterministically. Depth is
    /// checked first, so the user is told about the length rather than a loop far down the chain.
    #[test]
    fn an_over_deep_chain_that_also_cycles_reports_the_depth() {
        let last = MAX_EXTEND_DEPTH + 2;
        let (_tmp, store) = store_with(&[]);
        for i in 0..=last {
            // The final link points back to p0, so the graph is cyclic as well as too deep.
            let next = if i == last { 0 } else { i + 1 };
            fs::write(
                store.profile_file(&format!("p{i}")),
                format!("extends = [\"p{next}\"]\n"),
            )
            .unwrap();
        }

        let err = flatten_profile(&store, "p0").unwrap_err();
        assert!(matches!(err, SkmError::ExtendTooDeep { .. }), "{err:?}");
    }

    /// A wide diamond graph is not a depth problem: `expanded` keeps it linear.
    #[test]
    fn a_wide_graph_within_the_depth_limit_is_accepted() {
        let (_tmp, store) = store_with(&[
            ("app", &["a", "b", "c"], &[]),
            ("a", &["base"], &[]),
            ("b", &["base"], &[]),
            ("c", &["base"], &[]),
            ("base", &[], &["shared"]),
        ]);

        assert_eq!(ids(&store, "app"), vec!["shared"]);
    }

    // ---- dangling references ------------------------------------------------------

    #[test]
    fn extending_a_missing_profile_names_both_profiles() {
        let (_tmp, store) = store_with(&[("app", &["gone"], &["own"])]);

        let err = flatten_profile(&store, "app").unwrap_err();
        match err {
            SkmError::ExtendNotFound { profile, missing } => {
                assert_eq!(profile, "app");
                assert_eq!(missing, "gone");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_missing_starting_profile_is_still_a_plain_not_found() {
        let (_tmp, store) = store_with(&[]);
        let err = flatten_profile(&store, "nope").unwrap_err();
        assert!(
            matches!(err.leaf(), SkmError::ProfileNotFound(_)),
            "{err:?}"
        );
    }

    /// `extends` is a new input that gets turned into a file path (`<store>/.skm/profiles/
    /// <name>.toml`), so traversal has to be rejected before it is joined.
    #[test]
    fn extends_cannot_escape_the_profiles_directory() {
        for hostile in [
            "../../etc/passwd",
            "..",
            "/etc/passwd",
            "sub/dir",
            ".hidden",
            ".skm",
        ] {
            let (_tmp, store) = store_with(&[]);
            fs::write(
                store.profile_file("app"),
                format!("extends = [\"{hostile}\"]\n"),
            )
            .unwrap();

            let err = flatten_profile(&store, "app").unwrap_err();
            assert!(
                matches!(
                    err.leaf(),
                    SkmError::InvalidProfileName(_) | SkmError::ReservedName(_)
                ),
                "{hostile:?} was not rejected: {err:?}"
            );
        }
    }

    // ---- tree ---------------------------------------------------------------------

    fn tree_lines(store: &StorePaths, name: &str, disabled: &[&str]) -> (Vec<String>, Tree) {
        let disabled: HashSet<String> = disabled.iter().map(|d| d.to_string()).collect();
        let tree = build_tree(store, name, &disabled);
        (render_tree(&tree.root, false), tree)
    }

    #[test]
    fn a_diamond_renders_the_shared_subtree_once() {
        let (_tmp, store) = store_with(&[
            ("work", &["base", "infra"], &["pdf"]),
            ("base", &["shared"], &["docx"]),
            ("infra", &["shared"], &["tf"]),
            ("shared", &[], &["git"]),
        ]);

        let (lines, tree) = tree_lines(&store, "work", &[]);
        assert_eq!(
            lines,
            vec![
                "work",
                "├── pdf",
                "├── base",
                "│   ├── docx",
                "│   └── shared",
                "│       └── git",
                "└── infra",
                "    ├── tf",
                "    └── shared (*)",
            ]
        );
        assert_eq!(tree.resolved, 4, "pdf, docx, git, tf");
        assert_eq!(tree.disabled, 0);
        assert!(tree.error.is_none());
    }

    /// The resolved count must agree with the flat listing, or the two views contradict.
    #[test]
    fn the_resolved_count_matches_the_flat_listing() {
        let (_tmp, store) = store_with(&[
            ("work", &["base", "infra"], &["pdf"]),
            ("base", &["shared"], &["docx"]),
            ("infra", &["shared"], &["tf", "docx"]),
            ("shared", &[], &["git"]),
        ]);

        let (_lines, tree) = tree_lines(&store, "work", &[]);
        assert_eq!(
            tree.resolved,
            flatten_profile(&store, "work").unwrap().len()
        );
    }

    #[test]
    fn a_skill_an_earlier_profile_already_contributed_is_marked() {
        let (_tmp, store) = store_with(&[
            ("work", &["a", "b"], &[]),
            ("a", &[], &["git"]),
            ("b", &[], &["git"]),
        ]);

        let (lines, tree) = tree_lines(&store, "work", &[]);
        assert_eq!(
            lines,
            vec!["work", "├── a", "│   └── git", "└── b", "    └── git (*)"]
        );
        assert_eq!(tree.resolved, 1, "one placement, shown twice");
    }

    #[test]
    fn disabled_skills_are_marked_and_counted_separately() {
        let (_tmp, store) = store_with(&[("work", &[], &["git", "docx"])]);

        let (lines, tree) = tree_lines(&store, "work", &["git"]);
        assert_eq!(lines, vec!["work", "├── git (disabled)", "└── docx"]);
        assert_eq!(tree.resolved, 2);
        assert_eq!(tree.disabled, 1, "resolves but is never wired");
    }

    // ---- tree on a broken graph ---------------------------------------------------

    /// The tree is the view you want when `use-profiles` refuses a profile, so it renders what it
    /// can and marks the break instead of giving up.
    #[test]
    fn a_missing_profile_is_marked_and_siblings_still_render() {
        let (_tmp, store) =
            store_with(&[("work", &["gone", "ok"], &["pdf"]), ("ok", &[], &["git"])]);

        let (lines, tree) = tree_lines(&store, "work", &[]);
        assert_eq!(
            lines,
            vec![
                "work",
                "├── pdf",
                "├── gone (not found)",
                "└── ok",
                "    └── git",
            ]
        );
        assert_eq!(tree.resolved, 2, "the healthy branch still resolves");
        assert!(matches!(tree.error, Some(SkmError::ExtendNotFound { .. })));
    }

    #[test]
    fn a_cycle_is_marked_where_it_closes() {
        let (_tmp, store) = store_with(&[("a", &["b"], &["one"]), ("b", &["a"], &["two"])]);

        let (lines, tree) = tree_lines(&store, "a", &[]);
        assert_eq!(
            lines,
            vec!["a", "├── one", "└── b", "    ├── two", "    └── a (cycle)"]
        );
        assert!(matches!(tree.error, Some(SkmError::ExtendCycle(_))));
    }

    #[test]
    fn an_over_deep_chain_is_marked_at_the_limit() {
        let (_tmp, store) = store_with(&[]);
        let last = MAX_EXTEND_DEPTH + 1;
        for i in 0..=last {
            let extends = if i < last {
                format!("extends = [\"p{}\"]\n", i + 1)
            } else {
                String::new()
            };
            fs::write(store.profile_file(&format!("p{i}")), extends).unwrap();
        }

        let (lines, tree) = tree_lines(&store, "p0", &[]);
        assert!(matches!(tree.error, Some(SkmError::ExtendTooDeep { .. })));
        // The offending node is the one past the limit; everything above it still rendered.
        let tail = lines.last().unwrap();
        assert!(tail.contains(&format!("p{last} (too deep)")), "{lines:#?}");
        assert_eq!(lines.len(), last + 1, "p0..=p{last} on their own lines");
    }

    /// A TOML parse error is multi-line with caret art. Putting it in a node label would break
    /// the tree's shape, so notes stay a closed set of short labels and the detail rides on
    /// `Tree::error`, which the caller prints after the tree.
    #[test]
    fn an_unreadable_profile_gets_a_short_note_not_the_error_text() {
        let (_tmp, store) = store_with(&[("work", &["broken"], &["pdf"])]);
        fs::write(store.profile_file("broken"), "this is not valid toml =\n").unwrap();

        let (lines, tree) = tree_lines(&store, "work", &[]);
        assert_eq!(lines, vec!["work", "├── pdf", "└── broken (unreadable)"]);
        for line in &lines {
            assert!(
                !line.contains('\n'),
                "a newline would corrupt the tree: {line:?}"
            );
        }
        assert!(matches!(
            tree.error.as_ref().unwrap().leaf(),
            SkmError::InvalidProfile { .. }
        ));
    }

    /// `build_tree` and `flatten_profile` are two walks over the same graph, kept separate
    /// because one must refuse a broken graph and the other must render it. That duplication can
    /// drift, so pin them against each other across several shapes.
    #[test]
    fn the_tree_and_the_flat_listing_agree_on_every_healthy_shape() {
        let shapes: Vec<Vec<Shape>> = vec![
            // Plain chain.
            vec![
                ("a", &["b"], &["s1"]),
                ("b", &["c"], &["s2"]),
                ("c", &[], &["s3"]),
            ],
            // Diamond over a shared base.
            vec![
                ("a", &["l", "r"], &["s1"]),
                ("l", &["base"], &["s2"]),
                ("r", &["base"], &["s3"]),
                ("base", &[], &["s4"]),
            ],
            // Same skill declared at several levels.
            vec![
                ("a", &["b"], &["dup"]),
                ("b", &["c"], &["dup", "s2"]),
                ("c", &[], &["dup"]),
            ],
            // Meta profile with no skills of its own.
            vec![
                ("a", &["b", "c"], &[]),
                ("b", &[], &["s1"]),
                ("c", &[], &["s2"]),
            ],
            // An empty profile in the middle: a childless leaf that is NOT a skill.
            vec![
                ("a", &["empty", "b"], &["s1"]),
                ("empty", &[], &[]),
                ("b", &[], &["s2"]),
            ],
            // Wide fan-out onto one base.
            vec![
                ("a", &["x", "y", "z"], &[]),
                ("x", &["base"], &["s1"]),
                ("y", &["base"], &["s2"]),
                ("z", &["base"], &["s3"]),
                ("base", &[], &["s4"]),
            ],
        ];

        for (index, shape) in shapes.iter().enumerate() {
            let (_tmp, store) = store_with(shape);
            let flat = flatten_skill_ids(&store, "a").unwrap();
            let (_lines, tree) = tree_lines(&store, "a", &[]);

            assert_eq!(tree.resolved, flat.len(), "shape {index}: count");
            assert!(tree.error.is_none(), "shape {index}: {:?}", tree.error);

            // The skills the tree counts (unmarked leaves) must be exactly the flattened set.
            let counted = counted_skills(&tree.root);
            assert_eq!(counted, flat, "shape {index}: resolved set");
        }
    }

    /// Skill labels the tree counts: leaves with no `*` marker, in render order.
    fn counted_skills(node: &TreeNode) -> Vec<String> {
        let mut out = Vec::new();
        collect_counted(node, &mut out);
        out
    }

    fn collect_counted(node: &TreeNode, out: &mut Vec<String>) {
        for child in &node.children {
            if child.kind == NodeKind::Skill && !child.notes.contains(&"*") {
                out.push(child.label.clone());
            }
            collect_counted(child, out);
        }
    }

    #[test]
    fn a_profile_with_no_extends_renders_as_a_flat_list() {
        let (_tmp, store) = store_with(&[("solo", &[], &["a", "b"])]);
        let (lines, _) = tree_lines(&store, "solo", &[]);
        assert_eq!(lines, vec!["solo", "├── a", "└── b"]);
    }

    #[test]
    fn an_empty_profile_renders_as_a_single_line() {
        let (_tmp, store) = store_with(&[("empty", &[], &[])]);
        let (lines, tree) = tree_lines(&store, "empty", &[]);
        assert_eq!(lines, vec!["empty"]);
        assert_eq!(tree.resolved, 0);
    }

    #[test]
    fn a_missing_root_profile_is_marked_rather_than_panicking() {
        let (_tmp, store) = store_with(&[]);
        let (lines, tree) = tree_lines(&store, "nope", &[]);
        assert_eq!(lines, vec!["nope (not found)"]);
        assert!(matches!(
            tree.error.unwrap().leaf(),
            SkmError::ProfileNotFound(_)
        ));
    }

    // ---- graph queries ------------------------------------------------------------

    #[test]
    fn profiles_extending_lists_direct_extenders_only() {
        let (_tmp, store) = store_with(&[
            ("app", &["mid"], &[]),
            ("mid", &["base"], &[]),
            ("other", &["base"], &[]),
            ("base", &[], &["git"]),
        ]);

        assert_eq!(
            profiles_extending(&store, "base").unwrap(),
            vec!["mid", "other"]
        );
        assert_eq!(profiles_extending(&store, "mid").unwrap(), vec!["app"]);
        assert!(profiles_extending(&store, "app").unwrap().is_empty());
    }

    #[test]
    fn candidates_exclude_the_profile_itself_and_anything_that_reaches_it() {
        let (_tmp, store) = store_with(&[
            ("app", &[], &[]),
            ("mid", &["app"], &[]),
            ("top", &["mid"], &[]),
            ("unrelated", &[], &[]),
        ]);

        // `mid` extends app directly and `top` reaches it through mid: both would close a loop.
        assert_eq!(extend_candidates(&store, "app").unwrap(), vec!["unrelated"]);
        // Nothing reaches `unrelated`, so everything else is fair game.
        assert_eq!(
            extend_candidates(&store, "unrelated").unwrap(),
            vec!["app", "mid", "top"]
        );
    }
}
