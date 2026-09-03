mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::TempDir;

use common::*;

#[test]
fn use_profile_requires_project_setup() {
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

    cmd.args(["use-profile", "work"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(".skm.toml"));
}

#[test]
fn sync_requires_project_setup() {
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

    cmd.args(["sync"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(".skm.toml"));
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
