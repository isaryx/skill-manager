mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::TempDir;

use common::*;

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
        .args(["add-profile", "work"])
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
        .args(["add-profile", "work"])
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
        .args(["add-profile", "work"])
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
    assert!(!setup.contains("active = [\"work\"]"));
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
        .args(["add-profile", "work"])
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
        .args(["add-profile", "work"])
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
