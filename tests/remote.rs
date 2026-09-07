mod common;

use std::fs;
use std::process::Command as ProcessCommand;

use predicates::prelude::*;
use tempfile::TempDir;

use common::{git_available, git_init, init_project, with_env, write_profile};

fn git(repo: &std::path::Path, args: &[&str]) {
    let status = ProcessCommand::new("git")
        .current_dir(repo)
        .env("PRE_COMMIT_ALLOW_NO_CONFIG", "1")
        .args(args)
        .status()
        .expect("git spawn");
    assert!(status.success(), "git {:?} failed in {}", args, repo.display());
}

fn init_store(home: &std::path::Path, store: &std::path::Path) {
    with_env(home, store)
        .args(["init", "--agent", "claude-code", "--force"])
        .assert()
        .success();
}

fn write_skill_md(dir: &std::path::Path, name: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!(
            "---\nname: {name}\ndescription: Remote fixture skill {name}.\n---\n\n# {name}\n"
        ),
    )
    .unwrap();
}

fn create_fixture_repo() -> TempDir {
    let repo = TempDir::new().unwrap();
    write_skill_md(&repo.path().join("skills/deploy"), "deploy");
    write_skill_md(&repo.path().join("skills/lint"), "lint");
    git_init(repo.path());
    git(repo.path(), &["config", "user.email", "test@example.com"]);
    git(repo.path(), &["config", "user.name", "test"]);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-m", "init skills", "--quiet"]);
    repo
}

#[test]
fn repo_add_clones_local_path_and_lists_skills() {
    if !git_available() {
        return;
    }

    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let fixture = create_fixture_repo();

    init_store(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "repo",
            "add",
            fixture.path().to_str().unwrap(),
            "--name",
            "fixture",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("fixture/deploy"))
        .stdout(predicate::str::contains("fixture/lint"));

    with_env(home.path(), store.path())
        .args(["ls", "-s", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"fixture/deploy\""))
        .stdout(predicate::str::contains("\"fixture/lint\""));

    let link = store.path().join("fixture/deploy");
    assert!(link.is_symlink());
    assert!(link.join("SKILL.md").is_file());

    assert!(!home.path().join(".claude/skills/deploy").exists());
}

#[test]
fn repo_add_rejects_duplicate_name() {
    if !git_available() {
        return;
    }

    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let fixture = create_fixture_repo();
    init_store(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "repo",
            "add",
            fixture.path().to_str().unwrap(),
            "--name",
            "fixture",
        ])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args([
            "repo",
            "add",
            fixture.path().to_str().unwrap(),
            "--name",
            "fixture",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already registered"));
}

#[test]
fn repo_add_rejects_duplicate_url() {
    if !git_available() {
        return;
    }

    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let fixture = create_fixture_repo();
    init_store(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "repo",
            "add",
            fixture.path().to_str().unwrap(),
            "--name",
            "fixture-a",
        ])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args([
            "repo",
            "add",
            fixture.path().to_str().unwrap(),
            "--name",
            "fixture-b",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("URL already registered"));
}

#[test]
fn add_profile_pulls_before_wiring_new_remote_skill() {
    if !git_available() {
        return;
    }

    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let fixture = create_fixture_repo();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "repo",
            "add",
            fixture.path().to_str().unwrap(),
            "--name",
            "fixture",
        ])
        .assert()
        .success();

    write_skill_md(&fixture.path().join("skills/scan"), "scan");
    git(fixture.path(), &["add", "."]);
    git(fixture.path(), &["commit", "-m", "add scan", "--quiet"]);

    write_profile(store.path(), "scan-only", &["fixture/scan"]);
    with_env(home.path(), store.path())
        .args(["add-profile", "scan-only"])
        .assert()
        .success();

    let agent_link = home.path().join(".claude/skills/scan");
    assert!(agent_link.is_symlink());
}

#[test]
fn update_removes_library_symlink_when_skill_deleted_upstream() {
    if !git_available() {
        return;
    }

    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let fixture = create_fixture_repo();
    init_store(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "repo",
            "add",
            fixture.path().to_str().unwrap(),
            "--name",
            "fixture",
        ])
        .assert()
        .success();

    fs::remove_dir_all(fixture.path().join("skills/lint")).unwrap();
    git(fixture.path(), &["add", "-A", "."]);
    git(fixture.path(), &["commit", "-m", "remove lint", "--quiet"]);

    with_env(home.path(), store.path())
        .args(["update"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["ls", "-s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("fixture/lint").not());
}

#[test]
fn update_removes_stale_library_symlink_when_checkout_skill_deleted_locally() {
    if !git_available() {
        return;
    }

    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let fixture = create_fixture_repo();
    init_store(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "repo",
            "add",
            fixture.path().to_str().unwrap(),
            "--name",
            "fixture",
        ])
        .assert()
        .success();

    fs::remove_dir_all(store.path().join(".skm/remotes/fixture/skills/lint")).unwrap();

    with_env(home.path(), store.path())
        .args(["update"])
        .assert()
        .success();

    assert!(!store.path().join("fixture/lint").exists());
    assert!(store.path().join("fixture/deploy").is_symlink());
}

#[test]
fn repo_ls_json_lists_registered_repo() {
    if !git_available() {
        return;
    }

    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let fixture = create_fixture_repo();
    init_store(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "repo",
            "add",
            fixture.path().to_str().unwrap(),
            "--name",
            "fixture",
        ])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["repo", "ls", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"fixture\""))
        .stdout(predicate::str::contains("\"skill_count\": 2"));
}

#[test]
fn profile_and_sync_wire_remote_skills() {
    if !git_available() {
        return;
    }

    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let fixture = create_fixture_repo();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "repo",
            "add",
            fixture.path().to_str().unwrap(),
            "--name",
            "fixture",
        ])
        .assert()
        .success();

    write_profile(store.path(), "work", &["fixture/deploy"]);
    with_env(home.path(), store.path())
        .args(["add-profile", "work"])
        .assert()
        .success();

    let agent_link = home.path().join(".claude/skills/deploy");
    assert!(agent_link.is_symlink());
    assert!(agent_link.join("SKILL.md").is_file());

    with_env(home.path(), store.path())
        .args(["status", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"source_type\":\"remote\""));
}

#[test]
fn update_pulls_new_upstream_skill() {
    if !git_available() {
        return;
    }

    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let fixture = create_fixture_repo();
    init_store(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "repo",
            "add",
            fixture.path().to_str().unwrap(),
            "--name",
            "fixture",
        ])
        .assert()
        .success();

    write_skill_md(&fixture.path().join("skills/scan"), "scan");
    git(fixture.path(), &["add", "."]);
    git(fixture.path(), &["commit", "-m", "add scan", "--quiet"]);

    with_env(home.path(), store.path())
        .args(["update"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["ls", "-s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("fixture/scan"));
}

#[test]
fn sync_no_pull_skips_git_pull() {
    if !git_available() {
        return;
    }

    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let fixture = create_fixture_repo();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "repo",
            "add",
            fixture.path().to_str().unwrap(),
            "--name",
            "fixture",
        ])
        .assert()
        .success();

    write_profile(store.path(), "work", &["fixture/deploy"]);
    with_env(home.path(), store.path())
        .args(["add-profile", "work"])
        .assert()
        .success();

    write_skill_md(&fixture.path().join("skills/new-skill"), "new-skill");
    git(fixture.path(), &["add", "."]);
    git(fixture.path(), &["commit", "-m", "add new-skill", "--quiet"]);

    with_env(home.path(), store.path())
        .args(["sync", "--no-pull"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["ls", "-s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("fixture/new-skill").not());
}
