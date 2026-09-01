mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::TempDir;

use common::*;

#[test]
fn destroy_requires_a_project_setup() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["destroy", "--force"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("config file not found"));
}

#[test]
fn destroy_refuses_without_force_off_tty() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["destroy"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "refusing to destroy without --force",
        ));

    assert!(project.path().join(".skm.toml").is_file());
}

#[test]
fn destroy_warns_when_setup_has_no_active_profile() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["destroy", "--force"])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("warning:").and(predicate::str::contains("profile not found")),
        );

    assert!(!project.path().join(".skm.toml").exists());
}

#[test]
fn destroy_warns_when_active_profile_is_missing_from_the_store() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();
    fs::write(
        project.path().join(".skm.toml"),
        "version = 1\n[placement]\nagents = [\"claude-code\"]\n[profile]\nactive = \"gone\"\n",
    )
    .unwrap();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["destroy", "--force"])
        .assert()
        .success()
        .stderr(predicate::str::contains("profile not found: gone"));

    assert!(!project.path().join(".skm.toml").exists());
}

#[test]
fn destroy_force_unwires_links_removes_exclude_and_setup() {
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
    write_skill(&project.path().join(".claude/skills"), "project-skill");
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    let exclude = git_exclude(project.path());
    fs::write(
        &exclude,
        format!("*.local\n\n{}", fs::read_to_string(&exclude).unwrap()),
    )
    .unwrap();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["destroy", "--force"])
        .assert()
        .success();

    assert!(!project.path().join(".skm.toml").exists());
    assert!(!project.path().join(".claude/skills/docx").exists());
    assert!(project
        .path()
        .join(".claude/skills/project-skill/SKILL.md")
        .is_file());
    assert!(store.path().join("docx/SKILL.md").is_file());
    assert!(home.path().join(".skm.toml").is_file());

    let body = fs::read_to_string(&exclude).unwrap();
    assert_eq!(body, "*.local\n");
    assert!(!body.contains("skm-managed"), "{body}");
    assert!(!project.path().join(".gitignore").exists());
}

#[test]
fn destroy_dry_run_writes_nothing() {
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
        .args(["use-profile", "work"])
        .assert()
        .success();

    let setup = fs::read_to_string(project.path().join(".skm.toml")).unwrap();
    let exclude = fs::read_to_string(git_exclude(project.path())).unwrap();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["destroy", "--dry-run"])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("(dry-run) removing")
                .and(predicate::str::contains(".skm.toml")),
        );

    assert_eq!(
        fs::read_to_string(project.path().join(".skm.toml")).unwrap(),
        setup
    );
    assert!(project.path().join(".claude/skills/docx").is_symlink());
    assert_eq!(
        fs::read_to_string(git_exclude(project.path())).unwrap(),
        exclude
    );
}

#[test]
fn destroy_help_says_it_keeps_the_store() {
    skm().args(["destroy", "--help"]).assert().success().stdout(
        predicate::str::contains(".skm.toml")
            .and(predicate::str::contains("store-owned"))
            .and(predicate::str::contains("Does not delete the skill store"))
            .and(predicate::str::contains("every known")),
    );
}

#[test]
fn destroy_unwires_links_when_agents_list_is_empty() {
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
    write_skill(&project.path().join(".claude/skills"), "project-skill");
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    fs::write(
        project.path().join(".skm.toml"),
        "version = 1\n[placement]\nagents = []\n",
    )
    .unwrap();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["destroy", "--force"])
        .assert()
        .success();

    assert!(!project.path().join(".skm.toml").exists());
    assert!(!project.path().join(".claude/skills/docx").exists());
    assert!(project
        .path()
        .join(".claude/skills/project-skill/SKILL.md")
        .is_file());
    assert!(store.path().join("docx/SKILL.md").is_file());
}

#[test]
fn destroy_unwires_links_left_in_an_unlisted_agent_dir() {
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
        .args(["use-profile", "work"])
        .assert()
        .success();

    fs::write(
        project.path().join(".skm.toml"),
        "version = 1\n[placement]\nagents = [\"cursor\"]\n",
    )
    .unwrap();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["destroy", "--force"])
        .assert()
        .success();

    assert!(!project.path().join(".skm.toml").exists());
    assert!(!project.path().join(".claude/skills/docx").exists());
}

#[test]
fn destroy_warns_on_unknown_agent_and_still_unwires_known_dirs() {
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
        .args(["use-profile", "work"])
        .assert()
        .success();

    fs::write(
        project.path().join(".skm.toml"),
        "version = 1\n[placement]\nagents = [\"not-an-agent\"]\n",
    )
    .unwrap();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["destroy", "--force"])
        .assert()
        .success()
        .stderr(predicate::str::contains("unknown agent `not-an-agent`"));

    assert!(!project.path().join(".skm.toml").exists());
    assert!(!project.path().join(".claude/skills/docx").exists());
}
