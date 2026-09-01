// Compiled into every integration-test crate; not every helper is used in each one.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use assert_cmd::Command;
use tempfile::TempDir;

pub fn skm() -> Command {
    Command::cargo_bin("skm").unwrap()
}

pub fn with_env(home: &Path, store: &Path) -> Command {
    let mut cmd = skm();
    cmd.env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("SKM_STORE", store)
        .current_dir(home);
    cmd
}

pub fn write_skill(dir: &Path, name: &str) {
    let skill = dir.join(name);
    fs::create_dir_all(&skill).unwrap();
    fs::write(skill.join("SKILL.md"), format!("# {name}\n")).unwrap();
}

pub fn write_profile(store: &Path, name: &str, skills: &[&str]) {
    let profiles_dir = store.join(".skm/profiles");
    fs::create_dir_all(&profiles_dir).unwrap();
    let body: String = skills
        .iter()
        .map(|id| format!("[[skill]]\nid = \"{id}\"\n"))
        .collect();
    fs::write(profiles_dir.join(format!("{name}.toml")), body).unwrap();
}

pub fn write_disabled(store: &Path, ids: &[&str]) {
    let body = format!(
        "version = 1\n\nids = [{}]\n",
        ids.iter()
            .map(|id| format!("\"{id}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    fs::write(store.join(".skm/disabled.toml"), body).unwrap();
}

pub fn app_config_path(home: &Path) -> PathBuf {
    home.join(".config/skm/config.toml")
}

pub fn init_project(home: &Path, store: &Path) {
    with_env(home, store)
        .args(["init", "--agent", "claude-code"])
        .assert()
        .success();
}

pub fn git(project: &Path, args: &[&str]) {
    assert!(git_succeeds(project, args), "git {args:?} failed");
}

pub fn git_available() -> bool {
    ProcessCommand::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

pub fn git_succeeds(project: &Path, args: &[&str]) -> bool {
    ProcessCommand::new("git")
        .current_dir(project)
        .args(args)
        .status()
        .is_ok_and(|status| status.success())
}

pub fn git_exclude(project: &Path) -> PathBuf {
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

// ---- profile extends -------------------------------------------------------------

/// Profile file with an `extends` list, as `skm profile extend` would write it.
pub fn write_profile_extending(store: &Path, name: &str, extends: &[&str], skills: &[&str]) {
    let profiles_dir = store.join(".skm/profiles");
    fs::create_dir_all(&profiles_dir).unwrap();
    let list: Vec<String> = extends.iter().map(|e| format!("\"{e}\"")).collect();
    let mut body = format!("extends = [{}]\n", list.join(", "));
    for id in skills {
        body.push_str(&format!("\n[[skill]]\nid = \"{id}\"\n"));
    }
    fs::write(profiles_dir.join(format!("{name}.toml")), body).unwrap();
}

// ---- multiple target agents ---------------------------------------------------------------

/// `home` is both HOME and the project root in these tests, so an agent's project-level skills
/// directory hangs directly off it.
pub fn agent_link(home: &Path, agent_dir: &str, skill: &str) -> PathBuf {
    home.join(agent_dir).join("skills").join(skill)
}

pub fn setup_body(home: &Path) -> String {
    fs::read_to_string(home.join(".skm.toml")).unwrap()
}

/// Import one skill, put it in profile `work`, and activate it.
pub fn activate_docx(home: &Path, store: &Path) {
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
