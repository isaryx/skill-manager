mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::TempDir;

use common::*;

/// The `git_available()` gates in the local-exclude tests degrade to a silent pass when git is
/// missing, and the test harness has no stable way to report a skip — an `eprintln!` is captured
/// for passing tests, so those gates cannot announce themselves.
///
/// This is the one test that fails instead. A git-less machine gets a single clear signal rather
/// than a handful of quiet green ticks over assertions that never ran.
#[test]
fn git_is_available_for_the_local_exclude_tests() {
    assert!(
        git_available(),
        "git not found: the local-exclude tests cannot run and will report as passing"
    );
}

#[test]
fn local_excludes_are_isolated_between_projects_sharing_a_store() {
    if !git_available() {
        return;
    }
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project_a = TempDir::new().unwrap();
    let project_b = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "docx");
    write_profile(store.path(), "work", &["docx"]);

    for project in [&project_a, &project_b] {
        git_init(project.path());
        with_env(home.path(), store.path())
            .current_dir(project.path())
            .args(["init", "--agent", "claude-code"])
            .assert()
            .success();
        write_skill(&project.path().join(".claude/skills"), "project-skill");
        with_env(home.path(), store.path())
            .current_dir(project.path())
            .args(["add-profile", "work"])
            .assert()
            .success()
            .stderr(predicate::str::contains("updating local git exclude"));
        assert!(git_succeeds(
            project.path(),
            &["check-ignore", "--quiet", ".claude/skills/docx"]
        ));
        assert!(!git_succeeds(
            project.path(),
            &["check-ignore", "--quiet", ".claude/skills/project-skill"]
        ));
        // Patterns are anchored, so a same-named path elsewhere in the repo is untouched.
        write_skill(&project.path().join("vendor/skills"), "docx");
        assert!(!git_succeeds(
            project.path(),
            &["check-ignore", "--quiet", "vendor/skills/docx"]
        ));
    }

    let a_before = fs::read_to_string(git_exclude(project_a.path())).unwrap();
    let b = fs::read_to_string(git_exclude(project_b.path())).unwrap();
    assert!(a_before.contains("/.claude/skills/docx"), "{a_before}");
    assert!(b.contains("/.claude/skills/docx"), "{b}");
    assert!(!project_a.path().join(".gitignore").exists());
    assert!(!project_a.path().join(".claude/skills/.gitignore").exists());
    assert!(!project_b.path().join(".gitignore").exists());
    assert!(!project_b.path().join(".claude/skills/.gitignore").exists());

    with_env(home.path(), store.path())
        .current_dir(project_b.path())
        .args(["sync"])
        .assert()
        .success()
        .stderr(predicate::str::contains("updating local git exclude").not());
    assert_eq!(
        fs::read_to_string(git_exclude(project_a.path())).unwrap(),
        a_before
    );
}

#[test]
fn ignore_links_opt_out_removes_only_the_managed_exclude_block() {
    if !git_available() {
        return;
    }
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "docx");
    write_profile(store.path(), "work", &["docx"]);
    git_init(project.path());

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["add-profile", "work"])
        .assert()
        .success();

    let exclude = git_exclude(project.path());
    fs::write(
        &exclude,
        format!("*.local\n\n{}", fs::read_to_string(&exclude).unwrap()),
    )
    .unwrap();
    let setup = project.path().join(".skm.toml");
    let body = fs::read_to_string(&setup).unwrap().replace(
        "agents = [\"claude-code\"]",
        "agents = [\"claude-code\"]\nignore_links = false",
    );
    fs::write(setup, body).unwrap();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["sync"])
        .assert()
        .success()
        .stderr(predicate::str::contains("updating local git exclude"));

    let body = fs::read_to_string(exclude).unwrap();
    assert_eq!(body, "*.local\n");
    assert!(!project.path().join(".gitignore").exists());

    git(project.path(), &["add", ".claude/skills/docx"]);
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["doctor", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("link.tracked").not());
}

#[test]
fn sync_dry_run_does_not_change_local_exclude() {
    if !git_available() {
        return;
    }
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "docx");
    write_skill(store.path(), "git");
    write_profile(store.path(), "work", &["docx"]);
    git_init(project.path());

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["add-profile", "work"])
        .assert()
        .success();

    let exclude = git_exclude(project.path());
    let before = fs::read_to_string(&exclude).unwrap();
    write_profile(store.path(), "work", &["docx", "git"]);

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["sync", "--dry-run"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "(dry-run) updating local git exclude",
        ))
        .stderr(predicate::str::contains(
            "(dry-run) exclude + /.claude/skills/git",
        ));

    assert_eq!(fs::read_to_string(exclude).unwrap(), before);
    assert!(!project.path().join(".claude/skills/git").exists());
}

#[test]
fn sync_outside_git_does_not_create_a_gitignore() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "docx");
    write_profile(store.path(), "work", &["docx"]);

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["add-profile", "work"])
        .assert()
        .success();

    assert!(!project.path().join(".git").exists());
    assert!(!project.path().join(".gitignore").exists());
    assert!(!project.path().join(".claude/skills/.gitignore").exists());
}

#[test]
fn setup_agents_replaces_old_paths_in_local_exclude() {
    if !git_available() {
        return;
    }
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "docx");
    write_profile(store.path(), "work", &["docx"]);
    git_init(project.path());

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["add-profile", "work"])
        .assert()
        .success();
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["add-agent", "cursor"])
        .assert()
        .success();
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["remove-agent", "claude-code"])
        .assert()
        .success();

    let exclude = fs::read_to_string(git_exclude(project.path())).unwrap();
    assert!(exclude.contains("/.cursor/skills/docx"), "{exclude}");
    assert!(!exclude.contains("/.claude/skills/docx"), "{exclude}");
}

/// SPEC (`docs/SPEC.md`, local-exclude test table): a user-level skills directory is not inside
/// the project worktree, so syncing from a git project that has no project setup must leave that
/// project's exclude alone. `HOME` is not a repo here either, so nothing is written there.
///
/// This is the case `target.strip_prefix(&worktree.root)` guards: without it, a user-level sync
/// would write store paths from outside the repo into whichever repo the cwd happened to be in.
#[test]
fn user_level_sync_in_a_git_project_writes_no_local_exclude() {
    if !git_available() {
        return;
    }
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    // A user-level setup at `~/.skm.toml`, and deliberately none in the project.
    init_project(home.path(), store.path());
    write_skill(store.path(), "docx");
    write_profile(store.path(), "work", &["docx"]);
    git_init(project.path());
    assert!(!project.path().join(".skm.toml").exists());

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["add-profile", "work", "--user"])
        .assert()
        .success()
        .stderr(predicate::str::contains("updating local git exclude").not());

    // The link landed under HOME, so the project worktree has nothing to exclude.
    assert!(home.path().join(".claude/skills/docx").is_symlink());
    assert!(!project.path().join(".claude").exists());

    // Read leniently: some git versions ship a default `info/exclude`, and the invariant is that
    // skm did not add to it, not that the file is absent.
    let body = fs::read_to_string(git_exclude(project.path())).unwrap_or_default();
    assert!(!body.contains("skm-managed"), "{body}");
    assert!(!body.contains("docx"), "{body}");
    assert!(!project.path().join(".gitignore").exists());
    assert!(!home.path().join(".git").exists());
    assert!(!home.path().join(".gitignore").exists());
}

/// The no-write rule is not only "do not add a block". Empty patterns used to mean "remove the
/// managed block", which is the `ignore_links = false` opt-out. A user-level sync from a project
/// that already has a block must not take that path.
#[test]
fn user_level_sync_does_not_remove_an_existing_project_exclude() {
    if !git_available() {
        return;
    }
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "docx");
    write_profile(store.path(), "work", &["docx"]);
    git_init(project.path());

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["add-profile", "work"])
        .assert()
        .success();

    let exclude = git_exclude(project.path());
    let before = fs::read_to_string(&exclude).unwrap();
    assert!(before.contains("/.claude/skills/docx"), "{before}");

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["add-profile", "work", "--user"])
        .assert()
        .success()
        .stderr(predicate::str::contains("updating local git exclude").not());

    assert_eq!(fs::read_to_string(&exclude).unwrap(), before);
    assert!(home.path().join(".claude/skills/docx").is_symlink());
    assert!(project.path().join(".claude/skills/docx").is_symlink());
}

/// The links are the command; the exclude is a convenience. A block skm cannot parse is left
/// byte-for-byte alone and reconcile carries on, so a hand-mangled `info/exclude` cannot make skm
/// unusable in a repo. The warning has to name the file: in a linked worktree it lives under
/// `.git/worktrees/<name>/`, which nobody finds by guessing.
#[test]
fn a_malformed_exclude_block_warns_but_still_wires_links() {
    if !git_available() {
        return;
    }
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "docx");
    write_skill(store.path(), "git");
    write_profile(store.path(), "work", &["docx"]);
    git_init(project.path());

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["add-profile", "work"])
        .assert()
        .success();

    // Drop the END marker, the way a careless hand-edit would.
    let exclude = git_exclude(project.path());
    let broken: String = fs::read_to_string(&exclude)
        .unwrap()
        .lines()
        .filter(|line| *line != "# END skm-managed")
        .map(|line| format!("{line}\n"))
        .collect();
    fs::write(&exclude, &broken).unwrap();

    // A second skill, so the sync below has wiring left to do.
    write_profile(store.path(), "work", &["docx", "git"]);
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["sync"])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning:"))
        .stderr(predicate::str::contains("malformed"))
        .stderr(predicate::str::contains(
            exclude.to_string_lossy().to_string(),
        ))
        .stderr(predicate::str::contains("updating local git exclude").not());

    assert!(project.path().join(".claude/skills/git").is_symlink());
    assert!(project.path().join(".claude/skills/docx").is_symlink());
    assert_eq!(fs::read_to_string(&exclude).unwrap(), broken);
}

#[test]
fn local_exclude_covers_every_agent_directory() {
    if !git_available() {
        return;
    }
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    git_init(project.path());

    init_project(home.path(), store.path());
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();
    write_profile(store.path(), "work", &["docx"]);

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code,cursor"])
        .assert()
        .success();
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["add-profile", "work"])
        .assert()
        .success();

    let body = fs::read_to_string(git_exclude(project.path())).unwrap();
    assert!(body.contains("/.claude/skills/docx"), "{body}");
    assert!(body.contains("/.cursor/skills/docx"), "{body}");
}

#[test]
fn setup_agents_drops_the_removed_agent_from_local_exclude() {
    if !git_available() {
        return;
    }
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    git_init(project.path());

    init_project(home.path(), store.path());
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();
    write_profile(store.path(), "work", &["docx"]);

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code,cursor"])
        .assert()
        .success();
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["add-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["remove-agent", "claude-code"])
        .assert()
        .success();

    let body = fs::read_to_string(git_exclude(project.path())).unwrap();
    assert!(!body.contains("/.claude/skills/docx"), "{body}");
    assert!(body.contains("/.cursor/skills/docx"), "{body}");
}
