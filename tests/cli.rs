use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

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

fn git(project: &Path, args: &[&str]) {
    assert!(git_succeeds(project, args), "git {args:?} failed");
}

fn git_available() -> bool {
    ProcessCommand::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn git_succeeds(project: &Path, args: &[&str]) -> bool {
    ProcessCommand::new("git")
        .current_dir(project)
        .args(args)
        .status()
        .is_ok_and(|status| status.success())
}

fn git_exclude(project: &Path) -> PathBuf {
    let output = ProcessCommand::new("git")
        .current_dir(project)
        .args([
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "info/exclude",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    PathBuf::from(String::from_utf8(output.stdout).unwrap().trim())
}

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
fn agent_help_documents_skills_directory_per_agent() {
    skm().args(["init", "--help"]).assert().success().stdout(
        predicate::str::contains(".claude/skills")
            .and(predicate::str::contains(".cursor/skills"))
            .and(predicate::str::contains(".agents/skills"))
            .and(predicate::str::contains(".gemini/skills"))
            .and(predicate::str::contains(".github/skills")),
    );
}

#[test]
fn short_help_explains_engineering_workflow() {
    skm().arg("-h").assert().success().stdout(
        predicate::str::contains("one local store")
            .and(predicate::str::contains("skm init --agent claude-code"))
            .and(predicate::str::contains("SKM_STORE"))
            .and(predicate::str::contains("--json"))
            .and(predicate::str::contains("Pass --help")),
    );
}

#[test]
fn help_explains_engineering_workflow() {
    skm().arg("--help").assert().success().stdout(
        predicate::str::contains("canonical skill directories in a store")
            .and(predicate::str::contains("symlink"))
            .and(predicate::str::contains("skm init --agent claude-code"))
            .and(predicate::str::contains("SKM_STORE"))
            .and(predicate::str::contains("Exit codes"))
            .and(predicate::str::contains(
                "`sync`, `use-profile`, `skill rm`, `destroy` only",
            ))
            .and(predicate::str::contains(
                "--dry-run works with `sync`, `use-profile`, `skill rm`, and `destroy`",
            ))
            .and(predicate::str::contains(
                "https://github.com/isaryx/skill-manager",
            )),
    );
}

#[test]
fn command_help_documents_non_interactive_requirements() {
    skm()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "For non-interactive use, pass --agent",
        ));

    skm()
        .args(["skill", "rm", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Non-interactive use requires --force",
        ));

    skm()
        .args(["use-profile", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("[PROFILE]")
                .and(predicate::str::contains("choose interactively")),
        );
}

#[test]
fn profile_extend_help_says_it_creates_a_missing_profile() {
    skm()
        .args(["profile", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "extend  Choose which profiles this one inherits skills from (interactive; creates the profile if missing)",
        ));
}

#[test]
fn status_help_documents_read_only_report() {
    skm().args(["status", "--help"]).assert().success().stdout(
        predicate::str::contains("Read-only")
            .and(predicate::str::contains("non-skm"))
            .and(predicate::str::contains("Requires `./.skm.toml`"))
            .and(predicate::str::contains("--json")),
    );
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
        .stderr(
            predicate::str::contains(".skm.toml")
                .and(predicate::str::contains("setup-agents"))
                .and(predicate::str::contains("use-profile")),
        );
}

/// Off-TTY `init` without `--agent` would otherwise fail with NotATty from the picker. The
/// existing-setup check has to win so a second `init` never opens that list.
#[test]
fn init_refuses_existing_setup_before_prompting_for_agents() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["init"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("already exists")
                .and(predicate::str::contains("setup-agents"))
                .and(predicate::str::contains("TTY").not()),
        );
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
fn setup_agents_updates_setup_file() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["setup-agents", "--agent", "cursor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("target agents: cursor"));

    let content = fs::read_to_string(home.path().join(".skm.toml")).unwrap();
    assert!(content.contains("cursor"));
    assert!(!content.contains("claude-code"));
}

#[test]
fn setup_agents_reports_unchanged_agent() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "cursor"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["setup-agents", "--agent", "cursor"])
        .assert()
        .success()
        .stderr(predicate::str::contains("target agents unchanged: cursor"));
}

#[test]
fn setup_agents_requires_setup_file() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["setup-agents", "--agent", "cursor"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(".skm.toml"));
}

#[test]
fn setup_agents_requires_tty_without_agents() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["setup-agents"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("TTY"));
}

#[test]
fn setup_agents_rejects_invalid_agent() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["setup-agents", "--agent", "windsurf"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("invalid"));
}

#[test]
fn setup_agents_preserves_active_profile() {
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
        .args(["setup-agents", "--agent", "cursor"])
        .assert()
        .success();

    let content = fs::read_to_string(home.path().join(".skm.toml")).unwrap();
    assert!(content.contains("active = \"work\""));
    assert!(content.contains("cursor"));
}

#[test]
fn setup_agents_does_not_persist_on_sync_failure() {
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
        .args(["setup-agents", "--agent", "cursor"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in store"));

    let content = fs::read_to_string(home.path().join(".skm.toml")).unwrap();
    assert!(content.contains("claude-code"));
    assert!(!content.contains("cursor"));
}

#[test]
fn setup_agents_cleans_up_old_agent_symlinks() {
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
        .args(["setup-agents", "--agent", "cursor"])
        .assert()
        .success();

    assert!(
        fs::symlink_metadata(&old_link).is_err(),
        "expected switch-agent to remove the old agent's symlink at {}, but it is still present",
        old_link.display()
    );
}

#[test]
fn setup_agents_same_target_only_updates_agent_name() {
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
        .args(["setup-agents", "--agent", "generic"])
        .assert()
        .success()
        .stderr(predicate::str::contains("updating setup to generic"))
        .stderr(predicate::str::contains("syncing skills").not());

    assert!(
        fs::symlink_metadata(&link).is_ok(),
        "expected symlink to remain when only the agent name changes"
    );

    let content = fs::read_to_string(&setup_path).unwrap();
    assert!(content.contains("agents = [\"generic\"]"));
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
        .stdout(predicate::str::contains("Target agents:"))
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
        git(project.path(), &["init", "--quiet"]);
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
    git(project.path(), &["init", "--quiet"]);

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

/// Two tracked links and one untracked, so a `tracked_paths` that only ever reported its first
/// argument — or that reported every argument — would fail here. A single-link fixture cannot
/// tell those apart from the correct answer.
#[test]
fn doctor_warns_for_every_tracked_store_owned_link_and_no_others() {
    if !git_available() {
        return;
    }
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    for skill in ["docx", "git", "pdf"] {
        write_skill(store.path(), skill);
    }
    write_profile(store.path(), "work", &["docx", "git", "pdf"]);
    git(project.path(), &["init", "--quiet"]);

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
    // `-f` because the managed exclude block is doing its job.
    git(
        project.path(),
        &["add", "-f", ".claude/skills/docx", ".claude/skills/git"],
    );

    let output = with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["doctor", "--json"])
        .assert()
        .failure();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    assert_eq!(
        stdout.matches("\"code\":\"link.tracked\"").count(),
        2,
        "{stdout}"
    );
    assert!(stdout.contains("git rm --cached"), "{stdout}");
    for tracked in [".claude/skills/docx", ".claude/skills/git"] {
        assert!(stdout.contains(tracked), "{tracked} missing from {stdout}");
    }
    assert!(!stdout.contains(".claude/skills/pdf"), "{stdout}");
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
    git(project.path(), &["init", "--quiet"]);

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
        .args(["use-profile", "work"])
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
    git(project.path(), &["init", "--quiet"]);

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
    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["setup-agents", "--agent", "cursor"])
        .assert()
        .success();

    let exclude = fs::read_to_string(git_exclude(project.path())).unwrap();
    assert!(exclude.contains("/.cursor/skills/docx"), "{exclude}");
    assert!(!exclude.contains("/.claude/skills/docx"), "{exclude}");
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
fn use_profile_without_name_requires_a_tty() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_profile(store.path(), "work", &["docx"]);

    with_env(home.path(), store.path())
        .args(["use-profile"])
        .assert()
        .failure()
        .code(2)
        .stderr(
            predicate::str::contains("profile name is required")
                .and(predicate::str::contains("skm use-profile <profile>"))
                .and(predicate::str::contains("TTY")),
        );
}

#[test]
fn use_profile_without_name_reports_when_no_profiles_exist() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["use-profile"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("no profiles available")
                .and(predicate::str::contains("skm profile setup <name>")),
        );
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
fn store_cli_flag_overrides_env() {
    let home = TempDir::new().unwrap();
    let env_store = TempDir::new().unwrap();
    let flag_store = TempDir::new().unwrap();

    with_env(home.path(), env_store.path())
        .args([
            "--store",
            flag_store.path().to_str().unwrap(),
            "init",
            "--agent",
            "claude-code",
        ])
        .assert()
        .success();

    assert!(flag_store.path().join(".skm").is_dir());
    assert!(!env_store.path().join(".skm").exists());
}

#[test]
fn store_uses_app_config_when_env_unset() {
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

    skm()
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join(".config"))
        .env_remove("SKM_STORE")
        .current_dir(home.path())
        .args(["skill", "ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains("docx"));
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
fn add_command_removed() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["add", "./docx", "--copy"])
        .assert()
        .failure()
        .code(2);

    skm()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("[alias: add]").not());
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

#[test]
fn json_stdout_has_no_ansi_when_color_always() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["--color", "always", "doctor", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[^\x1b]*$").unwrap());
}

#[test]
fn doctor_json_reports_unknown_agent() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    fs::write(
        home.path().join(".skm.toml"),
        "version = 1\n[placement]\nagent = \"windsurf\"\n",
    )
    .unwrap();

    with_env(home.path(), store.path())
        .args(["doctor", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("config.unknown_agent"));
}

#[test]
fn doctor_json_reports_link_extra() {
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
    write_profile(store.path(), "work", &["docx"]);

    with_env(home.path(), store.path())
        .args(["doctor", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("link.extra"));
}

#[test]
fn use_profile_placement_name_collision_exits_two() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    for id in ["team/tdd", "other/tdd", "team__tdd"] {
        let dir = store.path().join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), format!("# {id}\n")).unwrap();
    }
    write_profile(
        store.path(),
        "clash",
        &["team/tdd", "other/tdd", "team__tdd"],
    );

    with_env(home.path(), store.path())
        .args(["use-profile", "clash"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("resolve conflict"));
}

#[test]
fn sync_unwires_when_all_profile_skills_are_disabled() {
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
    assert!(home.path().join(".claude/skills/docx").is_symlink());

    write_disabled(store.path(), &["docx"]);
    with_env(home.path(), store.path())
        .args(["sync"])
        .assert()
        .success();
    assert!(!home.path().join(".claude/skills/docx").exists());
}

#[test]
fn import_rejects_reserved_as_name() {
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
            "--as",
            ".hidden",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("reserved name"));
}

#[test]
fn import_rejects_invalid_as_name() {
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
            "--as",
            "Bad Name",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid skill id"));
}

#[test]
fn import_rejects_existing_destination() {
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
    with_env(home.path(), store.path())
        .args([
            "import",
            src.path().join("demo").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn sync_rejects_malformed_setup() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    fs::write(home.path().join(".skm.toml"), "not = toml {{").unwrap();

    with_env(home.path(), store.path())
        .args(["sync"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid config"));
}

#[test]
fn skill_ls_rejects_malformed_disabled_file() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    fs::write(store.path().join(".skm/disabled.toml"), "ids = not-a-list").unwrap();

    with_env(home.path(), store.path())
        .args(["skill", "ls"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("invalid disabled skills file")
                .or(predicate::str::contains("invalid skill store")),
        );
}

#[test]
fn gemini_cli_places_symlinks_under_gemini_skills() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");

    with_env(home.path(), store.path())
        .args(["init", "--agent", "gemini-cli"])
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

    assert!(home.path().join(".gemini/skills/docx").is_symlink());
}

// ---- profile extends -------------------------------------------------------------

/// Profile file with an `extends` list, as `skm profile extend` would write it.
fn write_profile_extending(store: &Path, name: &str, extends: &[&str], skills: &[&str]) {
    let profiles_dir = store.join(".skm/profiles");
    fs::create_dir_all(&profiles_dir).unwrap();
    let list: Vec<String> = extends.iter().map(|e| format!("\"{e}\"")).collect();
    let mut body = format!("extends = [{}]\n", list.join(", "));
    for id in skills {
        body.push_str(&format!("\n[[skill]]\nid = \"{id}\"\n"));
    }
    fs::write(profiles_dir.join(format!("{name}.toml")), body).unwrap();
}

#[test]
fn use_profile_links_inherited_skills() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "docx");
    write_skill(store.path(), "git");
    write_profile(store.path(), "base", &["git"]);
    write_profile_extending(store.path(), "work", &["base"], &["docx"]);

    with_env(home.path(), store.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    let skills_dir = home.path().join(".claude/skills");
    assert!(skills_dir.join("docx").is_symlink(), "own skill");
    assert!(skills_dir.join("git").is_symlink(), "inherited skill");
}

#[test]
fn use_profile_accepts_a_profile_whose_skills_all_come_from_extends() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_profile(store.path(), "base", &["git"]);
    write_profile_extending(store.path(), "meta", &["base"], &[]);

    with_env(home.path(), store.path())
        .args(["use-profile", "meta"])
        .assert()
        .success();

    assert!(home.path().join(".claude/skills/git").is_symlink());
}

#[test]
fn editing_a_base_profile_changes_what_extenders_resolve_to() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_skill(store.path(), "tf");
    write_profile(store.path(), "base", &["git"]);
    write_profile_extending(store.path(), "work", &["base"], &[]);

    with_env(home.path(), store.path())
        .args(["use-profile", "work"])
        .assert()
        .success();
    assert!(home.path().join(".claude/skills/git").is_symlink());

    // `extends` is a live reference, so rewriting base and syncing must follow.
    write_profile(store.path(), "base", &["tf"]);
    with_env(home.path(), store.path())
        .args(["sync"])
        .assert()
        .success();

    let skills_dir = home.path().join(".claude/skills");
    assert!(skills_dir.join("tf").is_symlink(), "new base skill wired");
    assert!(!skills_dir.join("git").exists(), "old base skill unwired");
}

#[test]
fn profile_show_marks_where_each_skill_came_from() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "docx");
    write_skill(store.path(), "git");
    write_profile(store.path(), "base", &["git"]);
    write_profile_extending(store.path(), "work", &["base"], &["docx"]);

    with_env(home.path(), store.path())
        .args(["profile", "show", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("docx\n"))
        .stdout(predicate::str::contains("git (from base)"));
}

#[test]
fn profile_show_combines_origin_and_disabled_markers() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_disabled(store.path(), &["git"]);
    write_profile(store.path(), "base", &["git"]);
    write_profile_extending(store.path(), "work", &["base"], &[]);

    with_env(home.path(), store.path())
        .args(["profile", "show", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("git (from base, disabled)"));
}

#[test]
fn profile_rm_refuses_while_another_profile_extends_it() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_profile(store.path(), "base", &["git"]);
    write_profile_extending(store.path(), "work", &["base"], &[]);

    with_env(home.path(), store.path())
        .args(["profile", "rm", "base"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("extended by work"));

    assert!(store.path().join(".skm/profiles/base.toml").is_file());
}

#[test]
fn profile_rm_allows_removing_a_profile_nothing_extends() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_profile(store.path(), "base", &["git"]);
    write_profile_extending(store.path(), "work", &["base"], &[]);

    with_env(home.path(), store.path())
        .args(["profile", "rm", "work"])
        .assert()
        .success();

    assert!(!store.path().join(".skm/profiles/work.toml").exists());
}

#[test]
fn use_profile_rejects_an_extend_cycle() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_profile_extending(store.path(), "a", &["b"], &["git"]);
    write_profile_extending(store.path(), "b", &["a"], &[]);

    with_env(home.path(), store.path())
        .args(["use-profile", "a"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("extend cycle"));
}

#[test]
fn use_profile_reports_a_missing_extended_profile() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_profile_extending(store.path(), "work", &["gone"], &["git"]);

    with_env(home.path(), store.path())
        .args(["use-profile", "work"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "profile `work` extends missing profile `gone`",
        ));
}

#[test]
fn doctor_reports_a_broken_extend_graph() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_profile(store.path(), "solid", &["git"]);
    write_profile_extending(store.path(), "broken", &["gone"], &["git"]);

    let output = with_env(home.path(), store.path())
        .args(["doctor", "--json"])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let issues = report["issues"].as_array().unwrap();

    assert!(
        issues
            .iter()
            .any(|issue| issue["code"] == "profile.extend_broken" && issue["profile"] == "broken"),
        "{report:#}"
    );
}

#[test]
fn doctor_does_not_call_an_extends_only_profile_empty() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_profile(store.path(), "base", &["git"]);
    write_profile_extending(store.path(), "meta", &["base"], &[]);

    let output = with_env(home.path(), store.path())
        .args(["doctor", "--json"])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let issues = report["issues"].as_array().unwrap();

    assert!(
        !issues
            .iter()
            .any(|issue| issue["code"] == "profile.empty" && issue["profile"] == "meta"),
        "{report:#}"
    );
}

#[test]
fn rewriting_profile_skills_preserves_extends() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_profile(store.path(), "base", &["git"]);
    write_profile_extending(store.path(), "work", &["base"], &["git"]);

    // `skill rm` rewrites every profile that references the skill; `extends` must survive.
    with_env(home.path(), store.path())
        .args(["skill", "rm", "git", "--force"])
        .assert()
        .success();

    let body = fs::read_to_string(store.path().join(".skm/profiles/work.toml")).unwrap();
    assert!(body.contains("extends"), "{body}");
    assert!(body.contains("base"), "{body}");
}

#[test]
fn profile_extend_requires_tty() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_profile(store.path(), "base", &["git"]);

    with_env(home.path(), store.path())
        .args(["profile", "extend", "work"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("TTY"));

    assert!(!store.path().join(".skm/profiles/work.toml").exists());
}

#[test]
fn extend_chain_past_the_depth_limit_is_rejected() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");

    // 10 profiles chained is 9 hops, past the limit of 8.
    for i in 0..10 {
        let next = format!("p{}", i + 1);
        let extends: &[&str] = if i < 9 { &[next.as_str()] } else { &[] };
        write_profile_extending(store.path(), &format!("p{i}"), extends, &["git"]);
    }

    with_env(home.path(), store.path())
        .args(["use-profile", "p0"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("deeper than 8"));
}

/// A skill listed both directly and by a base is one placement, not a duplicate-ID error.
#[test]
fn a_skill_in_both_a_profile_and_its_base_resolves_once() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_profile(store.path(), "base", &["git"]);
    write_profile_extending(store.path(), "work", &["base"], &["git"]);

    with_env(home.path(), store.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    assert!(home.path().join(".claude/skills/git").is_symlink());
    with_env(home.path(), store.path())
        .args(["profile", "show", "work"])
        .assert()
        .success()
        // Attributed to `work`, not `base`: a direct declaration wins.
        .stdout(predicate::str::contains("git\n"))
        .stdout(predicate::str::contains("from base").not());
}

/// Composing two bases that each contribute the same leaf name renames both placements.
///
/// This is the existing `__` disambiguation, but `extends` makes it reachable without the user
/// ever listing the two skills together: `eng` alone places `tdd`, while a profile extending
/// both places `engineering__tdd` and `ops__tdd`.
#[test]
fn colliding_leaves_from_two_bases_are_disambiguated() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    for dir in ["engineering", "ops"] {
        fs::create_dir_all(store.path().join(dir)).unwrap();
        write_skill(&store.path().join(dir), "tdd");
    }
    write_profile(store.path(), "eng", &["engineering/tdd"]);
    write_profile(store.path(), "opsp", &["ops/tdd"]);
    write_profile_extending(store.path(), "both", &["eng", "opsp"], &[]);

    let skills = home.path().join(".claude/skills");

    with_env(home.path(), store.path())
        .args(["use-profile", "eng"])
        .assert()
        .success();
    assert!(
        skills.join("tdd").is_symlink(),
        "unique leaf keeps the short name"
    );

    with_env(home.path(), store.path())
        .args(["use-profile", "both"])
        .assert()
        .success();
    assert!(skills.join("engineering__tdd").is_symlink());
    assert!(skills.join("ops__tdd").is_symlink());
    assert!(
        !skills.join("tdd").exists(),
        "short name gave way to the disambiguated pair"
    );
}

/// Two profiles that each resolve cleanly can still be unresolvable when combined.
///
/// `a/b` disambiguates to `a__b`, which collides with the unique leaf of `x/a__b`. Exit 2, as
/// for any resolve conflict. `skm profile show` is the way back: it names the origin of each
/// skill so the contributing base can be found.
#[test]
fn combining_bases_can_produce_an_unresolvable_collision() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    for (dir, name) in [("x", "a__b"), ("a", "b"), ("c", "b")] {
        fs::create_dir_all(store.path().join(dir)).unwrap();
        write_skill(&store.path().join(dir), name);
    }
    write_profile(store.path(), "p1", &["x/a__b"]);
    write_profile(store.path(), "p2", &["a/b", "c/b"]);
    write_profile_extending(store.path(), "both", &["p1", "p2"], &[]);

    for solo in ["p1", "p2"] {
        with_env(home.path(), store.path())
            .args(["use-profile", solo])
            .assert()
            .success();
    }

    with_env(home.path(), store.path())
        .args(["use-profile", "both"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("resolve conflict for `a__b`"));

    // The origin markers are what make this diagnosable.
    with_env(home.path(), store.path())
        .args(["profile", "show", "both"])
        .assert()
        .success()
        .stdout(predicate::str::contains("x/a__b (from p1)"))
        .stdout(predicate::str::contains("a/b (from p2)"));
}

/// An inherited disabled skill is reported as such and is not wired.
#[test]
fn inherited_disabled_skills_are_not_wired() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_skill(store.path(), "docx");
    write_disabled(store.path(), &["git"]);
    write_profile(store.path(), "base", &["git"]);
    write_profile_extending(store.path(), "work", &["base"], &["docx"]);

    with_env(home.path(), store.path())
        .args(["use-profile", "work"])
        .assert()
        .success();

    let skills = home.path().join(".claude/skills");
    assert!(skills.join("docx").is_symlink());
    assert!(
        !skills.join("git").exists(),
        "disabled inherited skill stays unwired"
    );
}

#[test]
fn profile_show_tree_renders_the_extend_graph() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    for name in ["pdf", "docx", "git", "tf"] {
        write_skill(store.path(), name);
    }
    write_profile(store.path(), "shared", &["git"]);
    write_profile_extending(store.path(), "base", &["shared"], &["docx"]);
    write_profile_extending(store.path(), "infra", &["shared"], &["tf"]);
    write_profile_extending(store.path(), "work", &["base", "infra"], &["pdf"]);

    let out = with_env(home.path(), store.path())
        .args(["--color", "never", "profile", "show", "work", "--tree"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(out).unwrap();

    assert_eq!(
        stdout,
        "work\n\
         ├── pdf\n\
         ├── base\n\
         │   ├── docx\n\
         │   └── shared\n\
         │       └── git\n\
         └── infra\n\
         \u{20}   ├── tf\n\
         \u{20}   └── shared (*)\n\
         \n\
         4 skills resolved\n",
        "{stdout}"
    );
}

#[test]
fn profile_show_tree_marks_disabled_skills_separately() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_skill(store.path(), "docx");
    write_disabled(store.path(), &["git"]);
    write_profile(store.path(), "base", &["git"]);
    write_profile_extending(store.path(), "work", &["base"], &["docx"]);

    with_env(home.path(), store.path())
        .args(["--color", "never", "profile", "show", "work", "--tree"])
        .assert()
        .success()
        .stdout(predicate::str::contains("└── git (disabled)"))
        .stdout(predicate::str::contains(
            "2 skills resolved, 1 disabled and not wired",
        ));
}

/// The tree is the view you reach for when `use-profile` refuses a profile, so it renders the
/// graph and then fails with the same code the flat listing would.
#[test]
fn profile_show_tree_renders_a_broken_graph_then_fails() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "pdf");
    write_skill(store.path(), "git");
    write_profile(store.path(), "ok", &["git"]);
    write_profile_extending(store.path(), "work", &["gone", "ok"], &["pdf"]);

    // Plain listing: no output, just the error.
    with_env(home.path(), store.path())
        .args(["profile", "show", "work"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty());

    // --tree: the graph, then the same failure.
    with_env(home.path(), store.path())
        .args(["--color", "never", "profile", "show", "work", "--tree"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("├── gone (not found)"))
        .stdout(predicate::str::contains("└── ok"))
        .stdout(predicate::str::contains("│   └── git").not())
        .stdout(predicate::str::contains("2 skills resolved"))
        .stderr(predicate::str::contains(
            "profile `work` extends missing profile `gone`",
        ));
}

#[test]
fn profile_show_tree_and_flat_agree_on_the_skill_count() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    for name in ["a", "b", "c"] {
        write_skill(store.path(), name);
    }
    write_profile(store.path(), "one", &["a", "b"]);
    write_profile(store.path(), "two", &["b", "c"]);
    write_profile_extending(store.path(), "both", &["one", "two"], &[]);

    let flat = with_env(home.path(), store.path())
        .args(["profile", "show", "both"])
        .output()
        .unwrap();
    let flat_lines = String::from_utf8(flat.stdout).unwrap().lines().count();

    with_env(home.path(), store.path())
        .args(["--color", "never", "profile", "show", "both", "--tree"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "{flat_lines} skills resolved"
        )));
}

#[test]
fn profile_show_tree_exits_two_on_a_cycle_like_the_flat_listing() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_skill(store.path(), "git");
    write_profile_extending(store.path(), "a", &["b"], &["git"]);
    write_profile_extending(store.path(), "b", &["a"], &[]);

    for args in [
        vec!["profile", "show", "a"],
        vec!["profile", "show", "a", "--tree"],
    ] {
        with_env(home.path(), store.path())
            .args(&args)
            .assert()
            .code(2);
    }
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
    git(project.path(), &["init", "--quiet"]);
    assert!(!project.path().join(".skm.toml").exists());

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["use-profile", "work"])
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
    git(project.path(), &["init", "--quiet"]);

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

    let exclude = git_exclude(project.path());
    let before = fs::read_to_string(&exclude).unwrap();
    assert!(before.contains("/.claude/skills/docx"), "{before}");

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["use-profile", "work", "--user"])
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
    git(project.path(), &["init", "--quiet"]);

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

// ---- multiple target agents ---------------------------------------------------------------

/// `home` is both HOME and the project root in these tests, so an agent's project-level skills
/// directory hangs directly off it.
fn agent_link(home: &Path, agent_dir: &str, skill: &str) -> PathBuf {
    home.join(agent_dir).join("skills").join(skill)
}

fn setup_body(home: &Path) -> String {
    fs::read_to_string(home.join(".skm.toml")).unwrap()
}

/// Import one skill, put it in profile `work`, and activate it.
fn activate_docx(home: &Path, store: &Path) {
    let src = TempDir::new().unwrap();
    write_skill(src.path(), "docx");
    with_env(home, store)
        .args([
            "import",
            src.path().join("docx").to_str().unwrap(),
            "--copy",
        ])
        .assert()
        .success();
    write_profile(store, "work", &["docx"]);
    with_env(home, store)
        .args(["use-profile", "work"])
        .assert()
        .success();
}

#[test]
fn init_accepts_repeated_agent_flags_and_links_into_each() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code", "--agent", "cursor"])
        .assert()
        .success();

    assert!(setup_body(home.path()).contains("agents = [\"claude-code\", \"cursor\"]"));

    activate_docx(home.path(), store.path());

    assert!(agent_link(home.path(), ".claude", "docx").is_symlink());
    assert!(agent_link(home.path(), ".cursor", "docx").is_symlink());
}

#[test]
fn init_accepts_comma_separated_agents() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code,gemini-cli"])
        .assert()
        .success();

    assert!(setup_body(home.path()).contains("agents = [\"claude-code\", \"gemini-cli\"]"));
}

#[test]
fn init_drops_a_repeated_agent() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "cursor", "--agent", "cursor"])
        .assert()
        .success();

    assert!(setup_body(home.path()).contains("agents = [\"cursor\"]"));
}

/// Two ids naming the same directory must not both be placed into: the second pass would unwire
/// what the first had just wired.
#[test]
fn init_collapses_agents_that_share_a_directory() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "generic"])
        .assert()
        .success();
    // `codex` is a config-only alias of `generic`, so both name `.agents/skills`.
    fs::write(
        home.path().join(".skm.toml"),
        "version = 1\n[placement]\nagents = [\"generic\", \"codex\"]\n",
    )
    .unwrap();

    activate_docx(home.path(), store.path());

    assert!(agent_link(home.path(), ".agents", "docx").is_symlink());
}

#[test]
fn setup_agents_adds_an_agent_and_syncs_into_its_directory() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();
    activate_docx(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["setup-agents", "--agent", "claude-code,cursor"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "target agents: claude-code, cursor",
        ));

    assert!(agent_link(home.path(), ".claude", "docx").is_symlink());
    assert!(agent_link(home.path(), ".cursor", "docx").is_symlink());
}

#[test]
fn setup_agents_unwires_only_the_dropped_agent() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code,cursor"])
        .assert()
        .success();
    activate_docx(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["setup-agents", "--agent", "cursor"])
        .assert()
        .success();

    let dropped = agent_link(home.path(), ".claude", "docx");
    assert!(
        fs::symlink_metadata(&dropped).is_err(),
        "expected the dropped agent's link at {} to be removed",
        dropped.display()
    );
    assert!(agent_link(home.path(), ".cursor", "docx").is_symlink());
    assert!(setup_body(home.path()).contains("agents = [\"cursor\"]"));
}

/// Dropping an agent needs no sync: the directories that remain are already wired.
#[test]
fn setup_agents_skips_sync_when_only_dropping_agents() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code,cursor"])
        .assert()
        .success();
    activate_docx(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["setup-agents", "--agent", "cursor"])
        .assert()
        .success()
        .stderr(predicate::str::contains("syncing skills").not());
}

#[test]
fn switch_agent_still_works_as_an_alias() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["switch-agent", "--agent", "cursor"])
        .assert()
        .success();

    assert!(setup_body(home.path()).contains("agents = [\"cursor\"]"));
}

/// Setups written before multi-agent support say `agent = "..."`. They must keep working, and
/// the next write should leave the file in the current shape.
#[test]
fn legacy_single_agent_config_is_read_and_migrated_on_write() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    init_project(home.path(), store.path());
    fs::write(
        home.path().join(".skm.toml"),
        "version = 1\n[placement]\nagent = \"claude-code\"\n",
    )
    .unwrap();

    activate_docx(home.path(), store.path());

    assert!(agent_link(home.path(), ".claude", "docx").is_symlink());
    let body = setup_body(home.path());
    assert!(body.contains("agents = [\"claude-code\"]"), "{body}");
}

#[test]
fn status_lists_every_target_agent_with_its_skills() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code,cursor"])
        .assert()
        .success();
    activate_docx(home.path(), store.path());

    with_env(home.path(), store.path())
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Target agents:"))
        .stdout(predicate::str::contains(".claude/skills"))
        .stdout(predicate::str::contains(".cursor/skills"))
        .stdout(predicate::str::contains("docx"));
}

#[test]
fn status_json_reports_one_entry_per_agent() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code,cursor"])
        .assert()
        .success();
    activate_docx(home.path(), store.path());

    let output = with_env(home.path(), store.path())
        .args(["status", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let agents = report["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 2);
    assert_eq!(agents[0]["agent"], "claude-code");
    assert_eq!(agents[0]["skills_path"], ".claude/skills");
    assert_eq!(agents[0]["skills"][0]["name"], "docx");
    assert_eq!(agents[1]["agent"], "cursor");
    assert_eq!(agents[1]["skills"][0]["name"], "docx");
    assert_eq!(report["profile"], "work");
}

#[test]
fn doctor_json_reports_every_target_agent() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code,cursor"])
        .assert()
        .success();
    activate_docx(home.path(), store.path());

    let output = with_env(home.path(), store.path())
        .args(["doctor", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        report["agents"].as_array().unwrap(),
        &vec![
            serde_json::json!("claude-code"),
            serde_json::json!("cursor")
        ]
    );
}

#[test]
fn doctor_names_the_agent_whose_directory_holds_an_extra_link() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();

    with_env(home.path(), store.path())
        .args(["init", "--agent", "claude-code,cursor"])
        .assert()
        .success();
    activate_docx(home.path(), store.path());

    // A store-owned link the active profile does not ask for, in one agent's directory only.
    let extra = store.path().join("docx");
    std::os::unix::fs::symlink(&extra, agent_link(home.path(), ".cursor", "extra")).unwrap();

    let output = with_env(home.path(), store.path())
        .args(["doctor", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let extra_issues: Vec<&serde_json::Value> = report["issues"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|issue| issue["code"] == "link.extra")
        .collect();
    assert_eq!(extra_issues.len(), 1);
    assert_eq!(extra_issues[0]["agent"], "cursor");
}

#[test]
fn local_exclude_covers_every_agent_directory() {
    if !git_available() {
        return;
    }
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    git(project.path(), &["init", "--quiet"]);

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
        .args(["use-profile", "work"])
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
    git(project.path(), &["init", "--quiet"]);

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
        .args(["use-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .current_dir(project.path())
        .args(["setup-agents", "--agent", "cursor"])
        .assert()
        .success();

    let body = fs::read_to_string(git_exclude(project.path())).unwrap();
    assert!(!body.contains("/.claude/skills/docx"), "{body}");
    assert!(body.contains("/.cursor/skills/docx"), "{body}");
}

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
    git(project.path(), &["init", "--quiet"]);

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
    git(project.path(), &["init", "--quiet"]);

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
