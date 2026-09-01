mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::TempDir;

use common::*;

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
