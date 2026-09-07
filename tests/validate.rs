mod common;

use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

use common::*;

fn write_invalid_skill(dir: &std::path::Path, name: &str) {
    let skill = dir.join(name);
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), "# no frontmatter\n").unwrap();
}

fn write_valid_skill(dir: &std::path::Path, name: &str) {
    let skill = dir.join(name);
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(
        skill.join("SKILL.md"),
        format!(
            "---\nname: {name}\ndescription: Validates SKILL.md frontmatter for {name}.\n---\n\n# {name}\n"
        ),
    )
    .unwrap();
}

#[test]
fn skill_validate_reports_invalid_frontmatter() {
    let dir = TempDir::new().unwrap();
    write_invalid_skill(dir.path(), "demo");

    let skill = dir.path().join("demo");
    skm()
        .args(["skill", "validate"])
        .arg(&skill)
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::contains("issue(s)"))
        .stdout(predicate::str::contains("missing YAML frontmatter"));
}

#[test]
fn skill_validate_succeeds_for_valid_skill() {
    let dir = TempDir::new().unwrap();
    write_valid_skill(dir.path(), "demo");

    let skill = dir.path().join("demo");
    skm()
        .args(["skill", "validate"])
        .arg(&skill)
        .assert()
        .success()
        .stdout(predicate::str::contains("is valid"));
}

#[test]
fn skill_validate_json_output_for_ci() {
    let dir = TempDir::new().unwrap();
    write_invalid_skill(dir.path(), "demo");

    let skill = dir.path().join("demo");
    let output = skm()
        .args(["skill", "validate", "--json"])
        .arg(&skill)
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["valid"], false);
    assert!(json["issues"].as_array().unwrap().len() >= 1);
}

#[test]
fn import_warns_on_invalid_frontmatter_by_default() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_invalid_skill(src.path(), "demo");

    init_project(home.path(), store.path());

    let skill = src.path().join("demo");
    with_env(home.path(), store.path())
        .args(["import", "--copy"])
        .arg(&skill)
        .assert()
        .success()
        .stderr(predicate::str::contains("warning:"))
        .stderr(predicate::str::contains("missing YAML frontmatter"));
}

#[test]
fn import_strict_fails_on_invalid_frontmatter() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_invalid_skill(src.path(), "demo");

    init_project(home.path(), store.path());

    let skill = src.path().join("demo");
    with_env(home.path(), store.path())
        .args(["import", "--copy", "--strict"])
        .arg(&skill)
        .assert()
        .failure()
        .stderr(predicate::str::contains("skill spec validation failed"));
}

#[test]
fn sync_strict_fails_on_invalid_store_skill() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let src = TempDir::new().unwrap();
    write_invalid_skill(src.path(), "demo");

    init_project(home.path(), store.path());
    let skill = src.path().join("demo");
    with_env(home.path(), store.path())
        .args(["import", "--copy"])
        .arg(&skill)
        .assert()
        .success();

    write_profile(store.path(), "work", &["demo"]);
    with_env(home.path(), store.path())
        .args(["add-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["sync", "--strict"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("skill spec validation failed"));
}
