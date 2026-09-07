mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::TempDir;

use common::*;

fn write_described_skill(root: &std::path::Path, id: &str, description: &str) {
    let dir = id
        .split('/')
        .fold(root.to_path_buf(), |path, segment| path.join(segment));
    fs::create_dir_all(&dir).unwrap();
    let name = id.rsplit('/').next().unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\n"),
    )
    .unwrap();
}

#[test]
fn search_matches_all_terms_case_insensitively_in_ids_and_descriptions() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    write_described_skill(
        store.path(),
        "local-docs",
        "Build polished internal documentation",
    );
    write_described_skill(
        store.path(),
        "community/deploy",
        "Production release automation for remote services",
    );
    write_described_skill(store.path(), "unrelated", "Formats source files");
    write_disabled(store.path(), &["community/deploy"]);
    write_profile(store.path(), "work", &["community/deploy"]);

    with_env(home.path(), store.path())
        .arg("scan")
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["search", "PRODUCTION", "remote"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("community/deploy")
                .and(predicate::str::contains("disabled"))
                .and(predicate::str::contains("work"))
                .and(predicate::str::contains("Production release automation"))
                .and(predicate::str::contains("unrelated").not()),
        );

    with_env(home.path(), store.path())
        .args(["search", "LOCAL", "documentation"])
        .assert()
        .success()
        .stdout(predicate::str::contains("local-docs"));
}

#[test]
fn search_json_is_structured_and_scan_refreshes_descriptions() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    write_described_skill(store.path(), "docs", "Original documentation helper");
    with_env(home.path(), store.path())
        .arg("scan")
        .assert()
        .success();

    write_described_skill(store.path(), "docs", "Updated release notes helper");

    with_env(home.path(), store.path())
        .args(["search", "updated"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    with_env(home.path(), store.path())
        .arg("scan")
        .assert()
        .success();

    let output = with_env(home.path(), store.path())
        .args(["--json", "search", "UPDATED", "notes"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let rows: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        rows,
        serde_json::json!([{
            "id": "docs",
            "enabled": true,
            "profiles": [],
            "description": "Updated release notes helper"
        }])
    );
}

#[test]
fn repo_qualified_import_is_searchable_without_a_separate_scan() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    init_project(home.path(), store.path());
    write_described_skill(
        source.path(),
        "deploy",
        "Publishes services to production environments",
    );

    with_env(home.path(), store.path())
        .args([
            "import",
            source.path().join("deploy").to_str().unwrap(),
            "--copy",
            "--repo",
            "community",
        ])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["search", "publishes", "PRODUCTION"])
        .assert()
        .success()
        .stdout(predicate::str::contains("community/deploy"));
}

#[test]
fn search_lists_profiles_that_inherit_a_skill_via_extends() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    write_described_skill(
        store.path(),
        "base-skill",
        "Inherited skill for profile membership search test",
    );
    common::write_profile_extending(store.path(), "work", &["base"], &[]);
    common::write_profile(store.path(), "base", &["base-skill"]);

    with_env(home.path(), store.path())
        .arg("scan")
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["search", "inherited", "membership"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("base-skill")
                .and(predicate::str::contains("base"))
                .and(predicate::str::contains("work")),
        );
}

#[test]
fn search_with_no_matches_prints_a_hint_on_stderr() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    write_described_skill(store.path(), "docs", "Original documentation helper");
    with_env(home.path(), store.path())
        .arg("scan")
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["search", "nomatch"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("no matching skills"))
        .stderr(predicate::str::contains("skm scan"));
}

#[test]
fn search_survives_broken_profile_extend_graph() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    write_described_skill(store.path(), "docs", "Searchable documentation helper");
    let profiles_dir = store.path().join(".skm/profiles");
    fs::create_dir_all(&profiles_dir).unwrap();
    fs::write(profiles_dir.join("a.toml"), "extends = [\"b\"]\n").unwrap();
    fs::write(profiles_dir.join("b.toml"), "extends = [\"a\"]\n").unwrap();

    with_env(home.path(), store.path())
        .arg("scan")
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["search", "documentation"])
        .assert()
        .success()
        .stdout(predicate::str::contains("docs"))
        .stderr(predicate::str::contains("skipping profile"));
}
