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
fn repo_rm_removes_checkout_and_library_symlinks() {
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
        .args(["repo", "rm", "fixture", "--force"])
        .assert()
        .success();

    assert!(!store.path().join("fixture/deploy").exists());
    assert!(!store.path().join(".skm/remotes/fixture").exists());
    assert!(!store.path().join(".skm/repos/fixture.toml").exists());
}

#[test]
fn repo_rm_refuses_when_profile_references_skills() {
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
        .args(["repo", "rm", "fixture"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("referenced by profiles"));
}

#[test]
fn repo_rm_force_removes_profile_refs() {
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
        .args(["repo", "rm", "fixture", "--force"])
        .assert()
        .success()
        .stderr(predicate::str::contains("updated profiles: work"));

    let profile = fs::read_to_string(store.path().join(".skm/profiles/work.toml")).unwrap();
    assert!(!profile.contains("fixture/deploy"));

    with_env(home.path(), store.path())
        .args(["doctor"])
        .assert()
        .stdout(predicate::str::contains("profile.missing_ref").not());
}

#[test]
fn import_github_rejects_copy_flag() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_store(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["import", "github:owner/repo", "--copy"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("omit --copy"));
}

#[test]
fn import_github_rejects_as_flag() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_store(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["import", "github:owner/repo", "--as", "alias"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("omit --as"));
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

#[test]
fn repo_add_with_pin_checks_out_ref() {
    if !git_available() {
        return;
    }

    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let fixture = create_fixture_repo();
    init_store(home.path(), store.path());

    git(fixture.path(), &["checkout", "-b", "pinned", "--quiet"]);
    write_skill_md(&fixture.path().join("skills/pinned-only"), "pinned-only");
    git(fixture.path(), &["add", "."]);
    git(fixture.path(), &["commit", "-m", "pinned skill", "--quiet"]);
    git(fixture.path(), &["checkout", "-", "--quiet"]);

    with_env(home.path(), store.path())
        .args([
            "repo",
            "add",
            fixture.path().to_str().unwrap(),
            "--name",
            "fixture",
            "--pin",
            "pinned",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("fixture/pinned-only"));

    with_env(home.path(), store.path())
        .args(["repo", "pin", "fixture"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pinned"));
}

#[test]
fn repo_pin_clear_drops_branch_only_skills() {
    if !git_available() {
        return;
    }

    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let fixture = create_fixture_repo();
    init_store(home.path(), store.path());

    git(fixture.path(), &["checkout", "-b", "pinned", "--quiet"]);
    write_skill_md(&fixture.path().join("skills/pinned-only"), "pinned-only");
    git(fixture.path(), &["add", "."]);
    git(fixture.path(), &["commit", "-m", "pinned skill", "--quiet"]);
    git(fixture.path(), &["checkout", "-", "--quiet"]);

    with_env(home.path(), store.path())
        .args([
            "repo",
            "add",
            fixture.path().to_str().unwrap(),
            "--name",
            "fixture",
            "--pin",
            "pinned",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("fixture/pinned-only"));

    with_env(home.path(), store.path())
        .args(["repo", "pin", "fixture", "--clear"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["ls", "-s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("fixture/pinned-only").not())
        .stdout(predicate::str::contains("fixture/deploy"));
}

#[test]
fn repo_add_writes_per_skill_sync_meta() {
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

    let meta_path = store.path().join(".skm/meta/fixture/deploy.toml");
    assert!(meta_path.is_file(), "expected per-skill meta at {}", meta_path.display());
    let body = fs::read_to_string(meta_path).unwrap();
    assert!(body.contains("source_type = \"remote\""));
    assert!(body.contains("commit = "));
    assert!(body.contains("synced_at = "));
}

#[test]
fn doctor_reports_stale_remote_skill_after_checkout_changes() {
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

    let checkout = store.path().join(".skm/remotes/fixture");
    fs::write(
        checkout.join("skills/deploy/SKILL.md"),
        "---\nname: deploy\ndescription: Updated for stale doctor test.\n---\n\n# deploy\n",
    )
    .unwrap();
    git(checkout.as_path(), &["config", "user.email", "test@example.com"]);
    git(checkout.as_path(), &["config", "user.name", "test"]);
    git(checkout.as_path(), &["add", "."]);
    git(checkout.as_path(), &["commit", "-m", "change deploy", "--quiet"]);

    with_env(home.path(), store.path())
        .args(["doctor", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("remote.stale"))
        .stdout(predicate::str::contains("recorded_commit"))
        .stdout(predicate::str::contains("current_commit"));

    with_env(home.path(), store.path())
        .args(["update", "fixture"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["doctor", "--json"])
        .assert()
        .stdout(predicate::str::contains("remote.stale").not());
}

#[test]
fn sync_updates_remote_skill_synced_at() {
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

    let meta_path = store.path().join(".skm/meta/fixture/deploy.toml");
    let before = fs::read_to_string(&meta_path).unwrap();
    let tampered = before.replace(
        "synced_at = \"",
        "synced_at = \"2000-01-01T00:00:00+00:00",
    );
    fs::write(&meta_path, tampered).unwrap();

    with_env(home.path(), store.path())
        .args(["sync", "--no-pull"])
        .assert()
        .success();

    let after = fs::read_to_string(&meta_path).unwrap();
    assert!(
        !after.contains("2000-01-01T00:00:00+00:00"),
        "sync should refresh synced_at in per-skill meta"
    );
    assert!(after.contains("synced_at = "));
}
