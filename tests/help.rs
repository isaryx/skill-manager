mod common;

use predicates::prelude::*;
use tempfile::TempDir;

use common::*;

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
