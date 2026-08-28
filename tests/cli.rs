use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn skm() -> Command {
    Command::cargo_bin("skm").unwrap()
}

fn with_env(home: &Path, store: &Path) -> Command {
    let mut cmd = skm();
    cmd.env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("SKM_STORE", store)
        .current_dir(home);
    cmd
}

fn write_skill(dir: &Path, name: &str) {
    let skill = dir.join(name);
    fs::create_dir_all(&skill).unwrap();
    fs::write(skill.join("SKILL.md"), format!("# {name}\n")).unwrap();
}

fn write_profile(store: &Path, name: &str, skills: &[&str]) {
    let profiles_dir = store.join(".skm/profiles");
    fs::create_dir_all(&profiles_dir).unwrap();
    let body: String = skills
        .iter()
        .map(|id| format!("[[skill]]\nid = \"{id}\"\n"))
        .collect();
    fs::write(profiles_dir.join(format!("{name}.toml")), body).unwrap();
}

fn write_disabled(store: &Path, ids: &[&str]) {
    let body = format!(
        "version = 1\n\nids = [{}]\n",
        ids.iter()
            .map(|id| format!("\"{id}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    fs::write(store.join(".skm/disabled.toml"), body).unwrap();
}

fn app_config_path(home: &Path) -> PathBuf {
    home.join(".config/skm/config.toml")
}

fn init_project(home: &Path, store: &Path) {
    with_env(home, store)
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();
}

#[test]
fn init_creates_store_and_setup() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    assert!(store.path().join(".skm/profiles").is_dir());
    assert!(store.path().join(".skm/meta").is_dir());
    assert!(store.path().join(".skm/index.db").is_file());
    assert!(home.path().join(".skm.toml").is_file());
    assert!(!fs::read_to_string(home.path().join(".skm.toml"))
        .unwrap()
        .contains("[store]"));

    let config = fs::read_to_string(app_config_path(home.path())).unwrap();
    assert!(config.contains(&store.path().to_string_lossy().to_string()));
}

#[test]
fn init_reuses_valid_store() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "keep-me");

    let project = TempDir::new().unwrap();
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    assert!(store.path().join("keep-me/SKILL.md").is_file());
    assert!(project.path().join(".skm.toml").is_file());
}

#[test]
fn init_rejects_invalid_store() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    fs::create_dir_all(store.path().join(".skm")).unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid skill store"));
}

#[test]
fn init_refuses_overwrite_without_force() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(".skm.toml"));
}

#[test]
fn init_force_overwrites() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "cursor"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["init", "--force", "--agent", "claude-code"])
        .assert()
        .success();

    let content = fs::read_to_string(home.path().join(".skm.toml")).unwrap();
    assert!(content.contains("claude-code"));
    assert!(!content.contains("cursor"));
}

#[test]
fn init_force_preserves_active_profile() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");

    init_project(home.path(), store.path());
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
        .args(["use-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["init", "--force", "--agent", "cursor"])
        .assert()
        .success();

    let content = fs::read_to_string(home.path().join(".skm.toml")).unwrap();
    assert!(content.contains("active = \"work\""));
    assert!(content.contains("cursor"));
    assert!(!content.contains("claude-code"));
}

#[test]
fn init_writes_skm_toml_in_cwd() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "cursor"])
        .assert()
        .success();

    assert!(project.path().join(".skm.toml").is_file());
}

#[test]
fn init_requires_tty_without_agents() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("TTY"));
}

#[test]
fn init_fails_when_agent_skills_dir_not_empty_without_accept_flag() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    let skill = home.path().join(".claude").join("skills").join("tdd");
    fs::create_dir_all(&skill).unwrap();
    fs::write(skill.join("SKILL.md"), "# tdd\n").unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("accept-existing-skills"));
}

#[test]
fn init_succeeds_with_accept_existing_skills_flag() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    let skill = home.path().join(".claude").join("skills").join("tdd");
    fs::create_dir_all(&skill).unwrap();
    fs::write(skill.join("SKILL.md"), "# tdd\n").unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code", "--accept-existing-skills"])
        .assert()
        .success();

    assert!(home.path().join(".skm.toml").is_file());
}

#[test]
fn init_succeeds_when_agent_skills_dir_empty() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    fs::create_dir_all(home.path().join(".claude").join("skills")).unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();
}

#[test]
fn init_user_and_project_subcommands_removed() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "user"])
        .assert()
        .failure()
        .code(2);

    with_env(home.path(), store.path())
        .args(["init", "project"])
        .assert()
        .failure()
        .code(2);

    with_env(home.path(), store.path())
        .args(["init", "store"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn switch_agent_updates_setup_file() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["switch-agent", "--agent", "cursor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("switched agent to cursor"));

    let content = fs::read_to_string(home.path().join(".skm.toml")).unwrap();
    assert!(content.contains("cursor"));
    assert!(!content.contains("claude-code"));
}

#[test]
fn switch_agent_reports_unchanged_agent() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "cursor"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["switch-agent", "--agent", "cursor"])
        .assert()
        .success()
        .stderr(predicate::str::contains("agent unchanged: cursor"));
}

#[test]
fn switch_agent_requires_setup_file() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["switch-agent", "--agent", "cursor"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(".skm.toml"));
}

#[test]
fn switch_agent_requires_tty_without_agents() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["switch-agent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("TTY"));
}

#[test]
fn switch_agent_rejects_invalid_agent() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["switch-agent", "--agent", "windsurf"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("invalid"));
}

#[test]
fn switch_agent_preserves_active_profile() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "demo");

    init_project(home.path(), store.path());
    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("demo").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();
    write_profile(store.path(), "work", &["demo"]);
    with_env(home.path(), store.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["switch-agent", "--agent", "cursor"])
        .assert()
        .success();

    let content = fs::read_to_string(home.path().join(".skm.toml")).unwrap();
    assert!(content.contains("active = \"work\""));
    assert!(content.contains("cursor"));
}

#[test]
fn switch_agent_does_not_persist_on_sync_failure() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["use-profile", "work"])
        .assert()
        .success();

    write_profile(store.path(), "work", &["nope"]);

    with_env(home.path(), store.path())
        .args(["switch-agent", "--agent", "cursor"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in store"));

    let content = fs::read_to_string(home.path().join(".skm.toml")).unwrap();
    assert!(content.contains("claude-code"));
    assert!(!content.contains("cursor"));
}

#[test]
fn switch_agent_cleans_up_old_agent_symlinks() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");

    init_project(home.path(), store.path());
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
        .args(["use-profile", "work"])
        .assert()
        .success();

    let old_link = home.path().join(".claude/skills/docx");
    assert!(
        fs::symlink_metadata(&old_link).is_ok(),
        "expected use-profile to wire the claude-code symlink first"
    );

    with_env(home.path(), store.path())
        .args(["switch-agent", "--agent", "cursor"])
        .assert()
        .success();

    assert!(
        fs::symlink_metadata(&old_link).is_err(),
        "expected switch-agent to remove the old agent's symlink at {}, but it is still present",
        old_link.display()
    );
}

#[test]
fn switch_agent_same_target_only_updates_agent_name() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");

    with_env(home.path(), store.path())
        .args(["init", "--agent", "generic"])
        .assert()
        .success();
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
        .args(["use-profile", "work"])
        .assert()
        .success();

    let setup_path = home.path().join(".skm.toml");
    let setup = fs::read_to_string(&setup_path)
        .unwrap()
        .replace("generic", "codex");
    fs::write(&setup_path, setup).unwrap();

    let link = home.path().join(".agents/skills/docx");
    assert!(fs::symlink_metadata(&link).is_ok());

    with_env(home.path(), store.path())
        .args(["switch-agent", "--agent", "generic"])
        .assert()
        .success()
        .stderr(predicate::str::contains("updating setup to generic"))
        .stderr(predicate::str::contains("syncing skills").not());

    assert!(
        fs::symlink_metadata(&link).is_ok(),
        "expected symlink to remain when only the agent name changes"
    );

    let content = fs::read_to_string(&setup_path).unwrap();
    assert!(content.contains("agent = \"generic\""));
    assert!(!content.contains("codex"));
}

#[test]
fn add_requires_init() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "demo");

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("demo").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("skm init"));
}

#[test]
fn scan_requires_init() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["scan"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("skm init"));
}

#[test]
fn scan_rebuilds_index_from_disk() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "demo");

    init_project(home.path(), store.path());
    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("demo").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    fs::remove_file(store.path().join(".skm/index.db")).unwrap();

    with_env(home.path(), store.path())
        .args(["scan"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["skill", "ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains("demo"));
}

#[test]
fn scan_verbose_succeeds() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["scan", "--verbose"])
        .assert()
        .success()
        .stderr(predicate::str::contains("indexed"));
}

#[test]
fn add_requires_copy_or_move() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "demo");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["import", src.path().join("demo").to_str().unwrap()])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn add_rejects_non_skill_dir() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    fs::create_dir_all(src.path().join("bad")).unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["import", src.path().join("bad").to_str().unwrap(), "--copy"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("SKILL.md"));
}

#[test]
fn add_copy_leaves_source() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "demo");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("demo").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    assert!(src.path().join("demo/SKILL.md").is_file());
    assert!(store.path().join("demo/SKILL.md").is_file());
}

#[test]
fn add_move_removes_source() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "demo");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("demo").to_str().unwrap(),
            "--move",
        ])
        .assert()
        .success();

    assert!(!src.path().join("demo").exists());
    assert!(store.path().join("demo/SKILL.md").is_file());
}

#[test]
fn profile_list_show_rm() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    write_profile(store.path(), "infra", &["docx", "git", "tf"]);

    with_env(home.path(), store.path())
        .args(["profile", "ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains("infra"));

    with_env(home.path(), store.path())
        .args(["profile", "show", "infra"])
        .assert()
        .success()
        .stdout(predicate::str::contains("docx"))
        .stdout(predicate::str::contains("git"))
        .stdout(predicate::str::contains("tf"));

    with_env(home.path(), store.path())
        .args(["profile", "rm", "infra"])
        .assert()
        .success();

    assert!(!store.path().join(".skm/profiles/infra.toml").exists());
}

#[test]
fn profile_setup_requires_tty() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["profile", "setup", "infra"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("TTY"));

    assert!(!store.path().join(".skm/profiles/infra.toml").exists());
}

#[test]
fn profile_add_command_removed() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["profile", "add", "infra", "docx"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn profile_show_reports_missing_profile() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["profile", "show", "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("profile not found"));
}

#[test]
fn profile_show_marks_active_profile() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["use-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["profile", "show", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("docx"))
        .stderr(predicate::str::contains("(active)"));
}

#[test]
fn skill_setup_requires_tty() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["skill", "setup"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("TTY"));
}

#[test]
fn disabled_skills_hidden_from_ls() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    write_skill(src.path(), "git");
    init_project(home.path(), store.path());

    for name in ["docx", "git"] {
        with_env(home.path(), store.path())
            .args(["import", src.path().join(name).to_str().unwrap(), "--copy"])
            .assert()
            .success();
    }

    write_disabled(store.path(), &["docx"]);

    with_env(home.path(), store.path())
        .args(["skill", "ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains("git"))
        .stdout(predicate::str::contains("docx").not());
}

#[test]
fn disabled_skill_in_profile_unwires_on_sync() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    write_skill(src.path(), "git");
    init_project(home.path(), store.path());

    for name in ["docx", "git"] {
        with_env(home.path(), store.path())
            .args(["import", src.path().join(name).to_str().unwrap(), "--copy"])
            .assert()
            .success();
    }

    write_profile(store.path(), "work", &["docx", "git"]);

    with_env(home.path(), store.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    assert!(home.path().join(".claude/skills/docx").is_symlink());
    assert!(home.path().join(".claude/skills/git").is_symlink());

    write_disabled(store.path(), &["docx"]);

    with_env(home.path(), store.path())
        .args(["sync"])
        .assert()
        .success();

    assert!(!home.path().join(".claude/skills/docx").exists());
    assert!(home.path().join(".claude/skills/git").is_symlink());

    with_env(home.path(), store.path())
        .args(["profile", "show", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("docx (disabled)"))
        .stdout(predicate::str::contains("git"));
}

#[test]
fn status_requires_project_setup() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    let mut cmd = skm();
    cmd.env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join(".config"))
        .env("SKM_STORE", store.path())
        .current_dir(project.path());

    cmd.args(["status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(".skm.toml"));
}

#[test]
fn use_and_sync_place_symlinks() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    write_skill(src.path(), "git");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();
    with_env(home.path(), store.path())
        .args(["import", src.path().join("git").to_str().unwrap(), "--copy"])
        .assert()
        .success();

    write_profile(store.path(), "infra", &["docx", "git"]);

    with_env(home.path(), store.path())
        .args(["use-profile", "infra"])
        .assert()
        .success();

    let link = home.path().join(".claude/skills/docx");
    assert!(link.is_symlink());
    let target = fs::read_link(&link).unwrap();
    assert!(target.is_absolute());
    assert!(store.path().join("docx").exists());

    with_env(home.path(), store.path())
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Target agent:"))
        .stdout(predicate::str::contains("Active profile:"))
        .stdout(predicate::str::contains("infra"))
        .stdout(predicate::str::contains("claude-code"))
        .stdout(predicate::str::contains(".claude/skills"))
        .stdout(predicate::str::contains("docx"))
        .stdout(predicate::str::contains("git"))
        .stdout(predicate::str::contains("symlink").not());
}

#[test]
fn use_switches_profiles_and_cleans_extras() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    write_skill(src.path(), "git");
    init_project(home.path(), store.path());

    for name in ["docx", "git"] {
        with_env(home.path(), store.path())
            .args(["import", src.path().join(name).to_str().unwrap(), "--copy"])
            .assert()
            .success();
    }

    write_profile(store.path(), "infra", &["docx", "git"]);
    write_profile(store.path(), "writing", &["docx"]);

    with_env(home.path(), store.path())
        .args(["use-profile", "infra"])
        .assert()
        .success();
    assert!(home.path().join(".claude/skills/git").is_symlink());

    with_env(home.path(), store.path())
        .args(["use-profile", "writing"])
        .assert()
        .success();
    assert!(!home.path().join(".claude/skills/git").exists());
    assert!(home.path().join(".claude/skills/docx").is_symlink());
}

#[test]
fn project_setup_uses_project_targets() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    write_profile(store.path(), "work", &["docx"]);

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    let link = project.path().join(".claude/skills/docx");
    assert!(link.is_symlink());
    assert!(!home.path().join(".claude/skills/docx").exists());
}

#[test]
fn use_user_flag_ignores_project_setup() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    write_profile(store.path(), "work", &["docx"]);

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["use-profile", "work", "-u"])
        .assert()
        .success();

    assert!(home.path().join(".claude/skills/docx").is_symlink());
    assert!(!project.path().join(".claude/skills/docx").exists());

    let user_setup = fs::read_to_string(home.path().join(".skm.toml")).unwrap();
    assert!(user_setup.contains("work"));
}

#[test]
fn sync_user_does_not_create_project_setup() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["use-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["sync", "--user"])
        .assert()
        .success();

    assert!(!project.path().join(".skm.toml").exists());
    assert!(home.path().join(".claude/skills/docx").is_symlink());
}

#[test]
fn sync_repairs_broken_symlink() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["use-profile", "work"])
        .assert()
        .success();

    fs::remove_file(home.path().join(".claude/skills/docx")).unwrap();

    with_env(home.path(), store.path())
        .args(["sync"])
        .assert()
        .success();

    assert!(home.path().join(".claude/skills/docx").is_symlink());
}

#[test]
fn sync_removes_extra_store_symlinks() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    write_skill(src.path(), "extra");
    init_project(home.path(), store.path());

    for name in ["docx", "extra"] {
        with_env(home.path(), store.path())
            .args(["import", src.path().join(name).to_str().unwrap(), "--copy"])
            .assert()
            .success();
    }

    write_profile(store.path(), "work", &["docx"]);
    with_env(home.path(), store.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            store.path().join("extra"),
            home.path().join(".claude/skills/extra"),
        )
        .unwrap();
    }

    with_env(home.path(), store.path())
        .args(["sync"])
        .assert()
        .success();

    assert!(!home.path().join(".claude/skills/extra").exists());
}

#[test]
fn sync_without_active_profile_fails() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["sync"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("active profile"));
}

#[test]
fn profile_rm_refuses_active_profile() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["use-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["profile", "rm", "work"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("active profile"));
}

#[test]
fn profile_rm_refuses_active_in_project_setup() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    write_profile(store.path(), "work", &["docx"]);

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["profile", "rm", "work"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("active profile"));
}

#[test]
fn ls_lists_pool_skills() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["skill", "ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains("docx"));
}

#[test]
fn init_rejects_unknown_agent_at_parse_time() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "windsurf"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("windsurf"));
}

#[test]
fn verbose_includes_error_context_chain() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["--verbose", "sync"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("syncing skills"));
}

#[test]
fn profile_name_traversal_rejected() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["profile", "setup", "../outside"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("reserved name")
                .or(predicate::str::contains("invalid profile name")),
        );
}

#[test]
fn use_profile_rejects_empty_profile() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    write_profile(store.path(), "empty", &[]);

    with_env(home.path(), store.path())
        .args(["use-profile", "empty"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("profile is empty"));
}

#[test]
fn use_does_not_persist_active_on_reconcile_failure() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    write_profile(store.path(), "missing-skills", &["nope"]);

    with_env(home.path(), store.path())
        .args(["use-profile", "missing-skills"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in store"));

    let setup = fs::read_to_string(home.path().join(".skm.toml")).unwrap();
    assert!(!setup.contains("missing-skills"));
}

#[test]
fn generic_agent_places_into_agents_skills() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");

    with_env(home.path(), store.path())
        .args(["init", "--agent", "generic"])
        .assert()
        .success();

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
        .args(["use-profile", "work"])
        .assert()
        .success();

    assert!(home.path().join(".agents/skills/docx").is_symlink());
}

#[test]
fn store_override_via_env() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    assert!(store.path().join(".skm").is_dir());
    assert!(home.path().join(".skm.toml").is_file());
    assert!(app_config_path(home.path()).is_file());
}

#[test]
fn index_rebuild_after_delete() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    write_profile(store.path(), "work", &["docx"]);

    fs::remove_file(store.path().join(".skm/index.db")).unwrap();

    with_env(home.path(), store.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    assert!(store.path().join(".skm/index.db").is_file());
    assert!(home.path().join(".claude/skills/docx").is_symlink());
}

#[test]
fn add_rejects_symlink_in_skill_tree() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "evil");
    init_project(home.path(), store.path());

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src.path().join("outside"), src.path().join("evil/link"))
            .unwrap();
        fs::create_dir_all(src.path().join("outside")).unwrap();
    }

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("evil").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("symlinks"));
}

#[test]
fn sync_skips_conflicted_placement() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    write_skill(src.path(), "other");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();
    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("other").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    write_profile(store.path(), "work", &["docx", "other"]);

    fs::create_dir_all(home.path().join(".claude/skills")).unwrap();
    fs::write(home.path().join(".claude/skills/docx"), "blocked").unwrap();

    with_env(home.path(), store.path())
        .args(["use-profile", "work"])
        .assert()
        .success()
        .stderr(predicate::str::contains("skipped docx (conflicted)"));

    assert!(home.path().join(".claude/skills/other").is_symlink());

    let docx = home.path().join(".claude/skills/docx");
    assert!(!docx.is_symlink());
    assert_eq!(fs::read_to_string(&docx).unwrap(), "blocked");
}

#[test]
fn sync_preserves_foreign_skill_not_in_profile() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    let review = home.path().join(".claude/skills/review");
    fs::create_dir_all(&review).unwrap();
    fs::write(review.join("SKILL.md"), "# review\n").unwrap();

    write_profile(store.path(), "work", &["docx"]);
    with_env(home.path(), store.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    assert!(review.join("SKILL.md").is_file());
    assert_eq!(
        fs::read_to_string(review.join("SKILL.md")).unwrap(),
        "# review\n"
    );
    assert!(home.path().join(".claude/skills/docx").is_symlink());
}

#[test]
fn use_profile_dry_run_reports_skipped_conflict() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    write_skill(src.path(), "other");
    init_project(home.path(), store.path());

    for name in ["docx", "other"] {
        with_env(home.path(), store.path())
            .args(["import", src.path().join(name).to_str().unwrap(), "--copy"])
            .assert()
            .success();
    }

    write_profile(store.path(), "work", &["docx", "other"]);
    fs::create_dir_all(home.path().join(".claude/skills")).unwrap();
    fs::write(home.path().join(".claude/skills/docx"), "blocked").unwrap();

    with_env(home.path(), store.path())
        .args(["use-profile", "work", "--dry-run"])
        .assert()
        .success()
        .stderr(predicate::str::contains("skipped docx (conflicted)"));

    let docx = home.path().join(".claude/skills/docx");
    assert!(!docx.is_symlink());
    assert_eq!(fs::read_to_string(&docx).unwrap(), "blocked");
    assert!(!home.path().join(".claude/skills/other").exists());

    let setup = fs::read_to_string(home.path().join(".skm.toml")).unwrap();
    assert!(!setup.contains("active = \"work\""));
}

#[test]
fn status_reports_conflicts() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    write_profile(store.path(), "work", &["docx"]);
    fs::create_dir_all(home.path().join(".claude/skills")).unwrap();
    fs::write(home.path().join(".claude/skills/docx"), "blocked").unwrap();

    with_env(home.path(), store.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Conflicts"))
        .stdout(predicate::str::contains("docx"));

    with_env(home.path(), store.path())
        .args(["status", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"conflicts\""))
        .stdout(predicate::str::contains("\"reason\":\"conflicted\""))
        .stdout(predicate::str::contains("\"store_id\":\"docx\""));
}

#[test]
fn doctor_json_reports_link_conflict() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["use-profile", "work"])
        .assert()
        .success();

    fs::remove_file(home.path().join(".claude/skills/docx")).unwrap();
    fs::write(home.path().join(".claude/skills/docx"), "blocked").unwrap();

    let output = with_env(home.path(), store.path())
        .args(["doctor", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let issues = report["issues"].as_array().unwrap();
    let conflict = issues
        .iter()
        .find(|issue| issue["code"] == "link.conflict")
        .expect("link.conflict issue");
    assert_eq!(conflict["severity"], "info");
    assert_eq!(conflict["skill"], "docx");
}

#[test]
fn doctor_reports_link_conflict() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["use-profile", "work"])
        .assert()
        .success();

    fs::remove_file(home.path().join(".claude/skills/docx")).unwrap();
    fs::write(home.path().join(".claude/skills/docx"), "blocked").unwrap();

    with_env(home.path(), store.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("link.conflict"));
}

#[test]
fn add_deeply_nested_bundle_preserves_tree_in_store() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let root = TempDir::new().unwrap();
    let bundle = root.path().join("engineering/backend");
    fs::create_dir_all(bundle.join("tdd")).unwrap();
    fs::write(bundle.join("tdd/SKILL.md"), "# tdd\n").unwrap();

    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["import", bundle.to_str().unwrap(), "--copy"])
        .assert()
        .success()
        .stdout(predicate::str::contains("backend/tdd"));

    assert!(store.path().join("backend/tdd/SKILL.md").is_file());
}

#[test]
fn add_bundle_preserves_tree_in_store() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let root = TempDir::new().unwrap();
    let bundle = root.path().join("engineering");

    fs::create_dir_all(bundle.join("tdd")).unwrap();
    fs::create_dir_all(bundle.join("code-review")).unwrap();
    fs::write(bundle.join("tdd/SKILL.md"), "# tdd\n").unwrap();
    fs::write(bundle.join("code-review/SKILL.md"), "# cr\n").unwrap();
    fs::write(bundle.join("README.md"), "# bundle\n").unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["import", bundle.to_str().unwrap(), "--copy"])
        .assert()
        .success()
        .stdout(predicate::str::contains("engineering/tdd"))
        .stdout(predicate::str::contains("engineering/code-review"));

    assert!(store.path().join("engineering/tdd/SKILL.md").is_file());
    assert!(store
        .path()
        .join("engineering/code-review/SKILL.md")
        .is_file());
    assert!(!store.path().join("tdd/SKILL.md").exists());
}

#[test]
fn add_bundle_accepts_as_for_store_name() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let root = TempDir::new().unwrap();
    let bundle = root.path().join("source-bundle");

    fs::create_dir_all(bundle.join("tdd")).unwrap();
    fs::write(bundle.join("tdd/SKILL.md"), "# tdd\n").unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            bundle.to_str().unwrap(),
            "--copy",
            "--as-name",
            "engineering",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("engineering/tdd"));

    assert!(store.path().join("engineering/tdd/SKILL.md").is_file());
}

#[test]
fn use_nested_bundle_skills_places_symlinks() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let root = TempDir::new().unwrap();
    let bundle = root.path().join("engineering");

    fs::create_dir_all(bundle.join("tdd")).unwrap();
    fs::create_dir_all(bundle.join("code-review")).unwrap();
    fs::write(bundle.join("tdd/SKILL.md"), "# tdd\n").unwrap();
    fs::write(bundle.join("code-review/SKILL.md"), "# cr\n").unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["import", bundle.to_str().unwrap(), "--copy"])
        .assert()
        .success();

    write_profile(
        store.path(),
        "test",
        &["engineering/tdd", "engineering/code-review"],
    );

    with_env(home.path(), store.path())
        .args(["use-profile", "test"])
        .assert()
        .success();

    let tdd = home.path().join(".claude/skills/tdd");
    let cr = home.path().join(".claude/skills/code-review");
    assert!(tdd.is_symlink());
    assert!(cr.is_symlink());
    assert!(fs::read_link(&tdd).unwrap().ends_with("engineering/tdd"));
    assert!(!home.path().join(".claude/skills/engineering").exists());
}

#[test]
fn use_removes_legacy_nested_symlinks() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let root = TempDir::new().unwrap();
    let bundle = root.path().join("engineering");

    fs::create_dir_all(bundle.join("tdd")).unwrap();
    fs::write(bundle.join("tdd/SKILL.md"), "# tdd\n").unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["import", bundle.to_str().unwrap(), "--copy"])
        .assert()
        .success();

    let legacy_dir = home.path().join(".claude/skills/engineering");
    fs::create_dir_all(&legacy_dir).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(store.path().join("engineering/tdd"), legacy_dir.join("tdd"))
        .unwrap();

    write_profile(store.path(), "test", &["engineering/tdd"]);

    with_env(home.path(), store.path())
        .args(["use-profile", "test"])
        .assert()
        .success();

    assert!(home.path().join(".claude/skills/tdd").is_symlink());
    assert!(!legacy_dir.exists());
}

#[test]
fn use_disambiguates_colliding_leaf_skill_names() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    for id in ["engineering/tdd", "other/tdd"] {
        let dir = store.path().join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), format!("# {id}\n")).unwrap();
    }

    write_profile(store.path(), "test", &["engineering/tdd", "other/tdd"]);

    with_env(home.path(), store.path())
        .args(["use-profile", "test"])
        .assert()
        .success();

    let eng = home.path().join(".claude/skills/engineering__tdd");
    let other = home.path().join(".claude/skills/other__tdd");
    assert!(eng.is_symlink());
    assert!(other.is_symlink());
}

#[test]
fn ls_skill_flag_lists_enabled_skills() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["ls", "--skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("docx"))
        .stdout(predicate::str::contains("skill/").not());
}

#[test]
fn skills_command_removed() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["skills", "ls"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn ls_lists_skills_and_profiles() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Profiles"))
        .stdout(predicate::str::contains("Skills"))
        .stdout(predicate::str::contains("skill/docx"))
        .stdout(predicate::str::contains("profile/work"));

    with_env(home.path(), store.path())
        .args(["ls", "-p"])
        .assert()
        .success()
        .stdout(predicate::str::contains("work"))
        .stdout(predicate::str::contains("skill/").not());

    with_env(home.path(), store.path())
        .args(["ls", "-s"])
        .assert()
        .success()
        .stdout(predicate::str::contains("docx"))
        .stdout(predicate::str::contains("profile/").not());
}

#[test]
fn import_add_alias_accepted() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["add", src.path().join("docx").to_str().unwrap(), "--copy"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["skill", "ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains("docx"));
}

#[test]
fn skill_rm_removes_skill_with_force() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    assert!(store.path().join("docx/SKILL.md").is_file());

    with_env(home.path(), store.path())
        .args(["skill", "rm", "docx", "--force"])
        .assert()
        .success()
        .stderr(predicate::str::contains("removed skill docx"));

    assert!(!store.path().join("docx").exists());

    with_env(home.path(), store.path())
        .args(["skill", "ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains("docx").not());
}

#[test]
fn skill_rm_not_found() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["skill", "rm", "missing", "--force"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("skill not found: missing"));
}

#[test]
fn skill_rm_refuses_without_force_non_tty() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["skill", "rm", "docx"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "refusing to remove without --force",
        ));

    assert!(store.path().join("docx/SKILL.md").is_file());
}

#[test]
fn skill_rm_removes_from_active_profile() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["use-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["skill", "rm", "docx", "--force"])
        .assert()
        .success()
        .stderr(predicate::str::contains("updated profiles: work"));

    assert!(!store.path().join("docx").exists());

    let profile_body = fs::read_to_string(store.path().join(".skm/profiles/work.toml")).unwrap();
    assert!(!profile_body.contains("docx"));

    let setup = fs::read_to_string(home.path().join(".skm.toml")).unwrap();
    assert!(!setup.contains("active = \"work\""));
}

#[test]
fn skill_rm_allowed_when_not_in_active_profile() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    write_skill(src.path(), "git");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();
    with_env(home.path(), store.path())
        .args(["import", src.path().join("git").to_str().unwrap(), "--copy"])
        .assert()
        .success();
    write_profile(store.path(), "work", &["docx"]);

    with_env(home.path(), store.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["skill", "rm", "git", "--force"])
        .assert()
        .success();

    assert!(!store.path().join("git").exists());
    assert!(store.path().join("docx/SKILL.md").is_file());
}

#[test]
fn use_command_removed() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["use", "work"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn group_command_not_available() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["group", "create", "devops"])
        .assert()
        .failure();
}

#[test]
fn init_copilot_cli_uses_github_skills_dir() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["init", "--agent", "copilot-cli"])
        .assert()
        .success();

    let content = fs::read_to_string(project.path().join(".skm.toml")).unwrap();
    assert!(content.contains("copilot-cli"));
}

#[test]
fn copilot_cli_places_symlinks_under_github_skills() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");

    with_env(home.path(), store.path())
        .args(["init", "--agent", "copilot-cli"])
        .assert()
        .success();

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
        .args(["use-profile", "work"])
        .assert()
        .success();

    let link = home.path().join(".github/skills/docx");
    assert!(link.is_symlink());
}

#[test]
fn doctor_clean_store_exits_zero() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No issues found").or(predicate::str::contains("issue")));
}

#[test]
fn doctor_reports_missing_profile_ref() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    write_profile(store.path(), "work", &["missing-skill"]);

    with_env(home.path(), store.path())
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("profile.missing_ref"));
}

#[test]
fn doctor_json_ok_matches_exit_code() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    write_profile(store.path(), "work", &["missing-skill"]);

    with_env(home.path(), store.path())
        .args(["doctor", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"ok\":false"))
        .stdout(predicate::str::contains("profile.missing_ref"));
}

#[test]
fn doctor_no_active_profile_skips_link_checks() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["doctor", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.no_active_profile"));
}

#[test]
fn doctor_reports_index_stale() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    // Add skill on disk without rebuilding index
    write_skill(store.path(), "extra");

    with_env(home.path(), store.path())
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("index.stale"));
}

#[test]
fn doctor_reports_broken_link() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["use-profile", "work"])
        .assert()
        .success();

    fs::remove_dir_all(store.path().join("docx")).unwrap();

    with_env(home.path(), store.path())
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(
            predicate::str::contains("link.broken")
                .or(predicate::str::contains("profile.missing_ref")),
        );
}

#[test]
fn status_json_outputs_fields() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["use-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["status", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"agent\":\"claude-code\""))
        .stdout(predicate::str::contains("\"profile\":\"work\""))
        .stdout(predicate::str::contains("\"skills_path\""))
        .stdout(predicate::str::contains("\"name\":\"docx\""))
        .stdout(predicate::str::contains("\"conflicts\":[]"));
}

#[test]
fn ls_json_lists_skills_and_profiles() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["ls", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"skills\":[\"docx\"]"))
        .stdout(predicate::str::contains("\"profiles\":[\"work\"]"));
}

#[test]
fn skill_ls_json_matches_ls_skill_json() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["skill", "ls", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("{\"skills\":[\"docx\"]}"));
}

#[test]
fn scan_adopts_copied_skills_without_meta() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    for id in ["local/a", "local/b"] {
        write_skill(store.path(), id);
    }

    with_env(home.path(), store.path())
        .args(["scan"])
        .assert()
        .success();

    assert!(store.path().join(".skm/meta/local.toml").is_file());

    with_env(home.path(), store.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("meta.missing").not());
}

#[test]
fn doctor_meta_missing_suggests_scan() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    write_skill(store.path(), "local/demo");

    with_env(home.path(), store.path())
        .args(["doctor"])
        .assert()
        .stdout(predicate::str::contains("meta.missing"))
        .stdout(predicate::str::contains("skm scan"));
}

#[test]
fn sync_adopts_copied_skills_without_meta() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    write_skill(store.path(), "local/demo");
    write_profile(store.path(), "work", &["docx"]);
    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();
    with_env(home.path(), store.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["sync"])
        .assert()
        .success();

    assert!(store.path().join(".skm/meta/local.toml").is_file());
}

#[test]
fn json_flag_rejected_on_unsupported_commands() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["init", "--json"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("--json"));

    with_env(home.path(), store.path())
        .args(["sync", "--json"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("--json"));
}

#[test]
fn dry_run_rejected_with_json() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["sync", "--dry-run", "--json"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("--dry-run"));
}

#[test]
fn sync_dry_run_does_not_create_symlinks() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["use-profile", "work"])
        .assert()
        .success();
    fs::remove_dir_all(home.path().join(".claude/skills")).ok();

    with_env(home.path(), store.path())
        .args(["sync", "--dry-run"])
        .assert()
        .success()
        .stderr(predicate::str::contains("(dry-run)"));

    let link = home.path().join(".claude/skills/docx");
    assert!(!link.exists());
}

#[test]
fn use_profile_dry_run_does_not_set_active() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["use-profile", "work", "--dry-run"])
        .assert()
        .success();

    let setup = fs::read_to_string(home.path().join(".skm.toml")).unwrap();
    assert!(!setup.contains("active = \"work\""));
}

#[test]
fn skill_rm_dry_run_leaves_store_intact() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["skill", "rm", "docx", "--dry-run"])
        .assert()
        .success()
        .stderr(predicate::str::contains("(dry-run)"));

    assert!(store.path().join("docx/SKILL.md").is_file());
}

#[test]
fn no_color_disables_ansi_on_sync_progress() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    init_project(home.path(), store.path());

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
        .args(["use-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .env("NO_COLOR", "1")
        .args(["sync"])
        .assert()
        .success()
        .stderr(predicate::str::is_match(r"^[^\x1b]*$").unwrap());
}
