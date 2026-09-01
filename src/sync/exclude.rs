use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::SkmError;
use crate::progress;

const BEGIN: &str = "# BEGIN skm-managed";
const END: &str = "# END skm-managed";

struct GitWorktree {
    root: PathBuf,
    exclude: PathBuf,
}

/// Rewrite the managed block to cover `targets`, each an agent skills directory paired with the
/// placement names it holds.
///
/// Takes every target at once because the block is a single list for the whole worktree: writing
/// it per directory would leave each agent's paths erasing the previous agent's.
pub(crate) fn sync_local_exclude(
    project_root: &Path,
    targets: &[(PathBuf, Vec<String>)],
    enabled: bool,
    dry_run: bool,
) -> Result<(), SkmError> {
    let Some(worktree) = locate_worktree(project_root)? else {
        return Ok(());
    };

    // Built only when enabled: with `ignore_links = false` the block is removed, and warning
    // about a path we were never going to list would be noise.
    let mut patterns: Vec<String> = Vec::new();
    let mut in_worktree = false;
    if enabled {
        for (target, placement_names) in targets {
            // A target outside the worktree (a user-level setup, say) has nothing git could
            // track, so it contributes no patterns.
            let Some(target_rel) = relative_to_worktree(target, &worktree.root) else {
                continue;
            };
            in_worktree = true;
            for name in placement_names {
                let path = target_rel.join(name);
                match escape_pattern(&path) {
                    Some(pattern) => patterns.push(pattern),
                    None => progress::warn(format!(
                        "not excluding `{}`: the path contains a line break, which a gitignore \
                         pattern cannot express; `git add` will not skip this link",
                        path.display()
                    )),
                }
            }
        }
        patterns.sort();
        patterns.dedup();
    }

    // Empty patterns plus an existing block means "remove the block". That is the opt-out
    // (`ignore_links = false`). It is not "this command had nothing in this worktree": a
    // user-level sync from a git project, or a cwd that only matches the toplevel after
    // canonicalize, would otherwise wipe a project block that a different setup still owns.
    if enabled && !in_worktree {
        return Ok(());
    }

    let existing = match fs::read_to_string(&worktree.exclude) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    // A block we cannot parse is left exactly as it is, and reconcile carries on. Failing here
    // would strand the caller: the links are the point of the command, the exclude is a
    // convenience, and `ignore_links = false` already supports wiring without one. The path is
    // spelled out because in a linked worktree it is under `.git/worktrees/<name>/`, which
    // nobody guesses.
    let Ok(updated) = replace_managed_block(&existing, &patterns) else {
        progress::warn(format!(
            "leaving local git exclude alone: {} has a malformed `{BEGIN}` / `{END}` block. \
             Fix or delete that block to let skm manage it again; until then store-owned skill \
             links are not excluded from `git add`.",
            worktree.exclude.display()
        ));
        return Ok(());
    };
    if updated == existing {
        return Ok(());
    }

    if dry_run {
        progress::step("(dry-run) updating local git exclude");
        let previous: BTreeSet<String> = managed_patterns(&existing).into_iter().collect();
        let desired: BTreeSet<String> = patterns.iter().cloned().collect();
        for pattern in previous.difference(&desired) {
            progress::step(format!("(dry-run) exclude - {pattern}"));
        }
        for pattern in desired.difference(&previous) {
            progress::step(format!("(dry-run) exclude + {pattern}"));
        }
        return Ok(());
    }

    if let Some(parent) = worktree.exclude.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(worktree.exclude, updated)?;
    progress::step("updating local git exclude");
    Ok(())
}

/// Which of `paths` git already tracks, in the order given.
///
/// One `git ls-files` for the whole set rather than one per path: a spawn costs roughly 25ms, and
/// `doctor` passes every store-owned link in the project. `ls-files` already lists only tracked
/// paths, so the per-path `--error-unmatch` that forced a spawn each time is not needed.
///
/// `--literal-pathspecs` keeps a `*` or `[` in a project directory name from being read as a
/// pathspec pattern; `-z` stops git quoting unusual names, so the output compares byte-for-byte
/// against what we passed in; `--full-name` pins the output to worktree-relative regardless of
/// where the command ran from.
pub(crate) fn tracked_paths(project_root: &Path, paths: &[PathBuf]) -> Vec<PathBuf> {
    let Some(worktree) = locate_worktree(project_root).ok().flatten() else {
        return Vec::new();
    };

    let relatives: Vec<PathBuf> = paths
        .iter()
        .filter_map(|path| relative_to_worktree(path, &worktree.root))
        .collect();
    if relatives.is_empty() {
        return Vec::new();
    }

    let Ok(output) = Command::new("git")
        .current_dir(&worktree.root)
        .args(["--literal-pathspecs", "ls-files", "-z", "--full-name", "--"])
        .args(&relatives)
        .stderr(Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    // Compared as paths, not strings, so Windows' two separators do not read as distinct names.
    let tracked: HashSet<PathBuf> = String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(PathBuf::from)
        .collect();

    paths
        .iter()
        .filter(|path| {
            relative_to_worktree(path, &worktree.root)
                .is_some_and(|relative| tracked.contains(&relative))
        })
        .cloned()
        .collect()
}

/// Worktree-relative path for `path`, or `None` when it is not inside this worktree.
///
/// Lexical `strip_prefix` first so a directory that does not exist yet still matches. Canonical
/// comparison is the fallback when cwd and `git rev-parse --show-toplevel` disagree by symlink.
fn relative_to_worktree(path: &Path, worktree_root: &Path) -> Option<PathBuf> {
    if let Ok(relative) = path.strip_prefix(worktree_root) {
        return Some(relative.to_path_buf());
    }
    let path = path.canonicalize().ok()?;
    let root = worktree_root.canonicalize().ok()?;
    path.strip_prefix(root).ok().map(PathBuf::from)
}

fn locate_worktree(project_root: &Path) -> Result<Option<GitWorktree>, SkmError> {
    let output = match Command::new("git")
        .current_dir(project_root)
        .args([
            "rev-parse",
            "--path-format=absolute",
            "--show-toplevel",
            "--git-path",
            "info/exclude",
        ])
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let Some(root) = lines.next() else {
        return Ok(None);
    };
    let Some(exclude) = lines.next() else {
        return Ok(None);
    };
    Ok(Some(GitWorktree {
        root: PathBuf::from(root),
        exclude: PathBuf::from(exclude),
    }))
}

/// The exclude file does not hold exactly one well-formed `BEGIN` … `END` pair, so there is no
/// way to rewrite the block without risking user lines. skm only ever writes a matched pair, so
/// reaching this means something else edited the file.
#[derive(Debug)]
struct MalformedBlock;

fn replace_managed_block(existing: &str, patterns: &[String]) -> Result<String, MalformedBlock> {
    let lines: Vec<&str> = existing.lines().collect();
    let begins: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == BEGIN).then_some(index))
        .collect();
    let ends: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == END).then_some(index))
        .collect();
    let range = match (begins.as_slice(), ends.as_slice()) {
        ([], []) => None,
        ([start], [end]) if start < end => Some((*start, *end)),
        _ => return Err(MalformedBlock),
    };
    if patterns.is_empty() && range.is_none() {
        return Ok(existing.to_string());
    }

    let mut kept = Vec::new();
    match range {
        Some((start, end)) => {
            kept.extend_from_slice(&lines[..start]);
            kept.extend_from_slice(&lines[end + 1..]);
        }
        None => kept.extend_from_slice(&lines),
    }
    while kept.last().is_some_and(|line| line.is_empty()) {
        kept.pop();
    }

    let mut output = kept.join("\n");
    if !patterns.is_empty() {
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        output.push_str(BEGIN);
        output.push('\n');
        output.push_str(&patterns.join("\n"));
        output.push('\n');
        output.push_str(END);
    }
    if !output.is_empty() {
        output.push('\n');
    }
    Ok(output)
}

fn managed_patterns(existing: &str) -> Vec<String> {
    let mut managed = false;
    let mut patterns = Vec::new();
    for line in existing.lines() {
        if line == BEGIN {
            managed = true;
            continue;
        }
        if managed && line == END {
            break;
        }
        if managed && !line.is_empty() {
            patterns.push(line.to_string());
        }
    }
    patterns
}

/// A worktree-relative path as a gitignore pattern anchored to the worktree root.
///
/// The leading `/` is the anchor. A pattern with no separator in it matches at every depth, so a
/// bare `docx` would also ignore `vendor/docx`; one with an interior separator is already
/// root-relative. Every adapter today nests its skills directory at least two segments deep, so
/// the interior separator is always there — the `/` makes the anchoring a property of this
/// function instead of a property of the adapter layout.
///
/// `None` for a path a gitignore pattern cannot express. Escaping runs before the anchor is
/// added, so the `/` is never itself escaped.
fn escape_pattern(path: &Path) -> Option<String> {
    let text = path.to_string_lossy();
    // gitignore is line-based and has no escape for a line break, so such a path has no pattern.
    // Emitting one anyway would split it across two patterns, and a component containing the END
    // marker would forge the block terminator.
    if text.contains('\n') || text.contains('\r') {
        return None;
    }
    let escaped = text
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace('*', "\\*")
        .replace('?', "\\?")
        .replace('#', "\\#")
        .replace('!', "\\!");
    Some(format!("/{escaped}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_block_preserves_user_content_and_can_be_removed() {
        let existing = "*.log\n\n# BEGIN skm-managed\nold\n# END skm-managed\nlocal.tmp\n";
        let updated = replace_managed_block(existing, &[".claude/skills/docx".into()]).unwrap();
        assert_eq!(
            updated,
            "*.log\n\nlocal.tmp\n\n# BEGIN skm-managed\n.claude/skills/docx\n# END skm-managed\n"
        );
        assert_eq!(
            replace_managed_block(&updated, &[]).unwrap(),
            "*.log\n\nlocal.tmp\n"
        );
    }

    #[test]
    fn malformed_managed_block_is_not_rewritten() {
        for broken in [
            "# BEGIN skm-managed\nkeep-me\n",
            "keep-me\n# END skm-managed\n",
            "# END skm-managed\n# BEGIN skm-managed\n",
            "# BEGIN skm-managed\na\n# END skm-managed\n# BEGIN skm-managed\nb\n# END skm-managed\n",
        ] {
            assert!(
                replace_managed_block(broken, &["new".into()]).is_err(),
                "{broken:?}"
            );
        }
    }

    /// A pattern with no separator matches at every depth, so anchoring is what keeps
    /// `/.claude/skills/docx` from also ignoring an unrelated `vendor/docx`.
    #[test]
    fn patterns_are_anchored_to_the_worktree_root() {
        assert_eq!(
            escape_pattern(Path::new(".claude/skills/docx")).unwrap(),
            "/.claude/skills/docx"
        );
        assert_eq!(escape_pattern(Path::new("docx")).unwrap(), "/docx");
    }

    #[test]
    fn glob_metacharacters_in_a_project_path_are_escaped() {
        assert_eq!(
            escape_pattern(Path::new("weird[1]*/skills/docx")).unwrap(),
            "/weird\\[1]\\*/skills/docx"
        );
    }

    /// gitignore has no escape for a line break, so there is no pattern to emit. Returning `None`
    /// makes the caller warn instead of writing a line that would split the block in two.
    #[test]
    fn a_path_containing_a_line_break_has_no_pattern() {
        assert!(escape_pattern(Path::new("a\nb/skills/docx")).is_none());
        assert!(escape_pattern(Path::new("a\r/skills/docx")).is_none());
    }

    #[test]
    fn relative_to_worktree_matches_a_lexical_prefix() {
        let root = Path::new("/tmp/proj");
        assert_eq!(
            relative_to_worktree(&root.join(".claude/skills"), root).as_deref(),
            Some(Path::new(".claude/skills"))
        );
        assert!(relative_to_worktree(Path::new("/elsewhere/.claude/skills"), root).is_none());
    }

    /// `git rev-parse --show-toplevel` may be the real path while `cwd` is a symlink to it.
    /// Lexical strip then fails and would look like "not in this worktree".
    #[cfg(unix)]
    #[test]
    fn relative_to_worktree_matches_through_a_symlink() {
        let tmp = tempfile::TempDir::new().unwrap();
        let real = tmp.path().join("real");
        fs::create_dir_all(real.join(".claude/skills")).unwrap();
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let relative = relative_to_worktree(&link.join(".claude/skills"), &real).unwrap();
        assert_eq!(relative, Path::new(".claude/skills"));
    }
}
