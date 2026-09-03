mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::TempDir;

use common::*;

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
fn setup_agents_requires_project_setup_in_foreign_dir() {
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

    cmd.args(["setup-agents", "--agent", "cursor"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(".skm.toml"));
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
