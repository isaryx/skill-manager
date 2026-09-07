mod common;

use predicates::prelude::*;
use tempfile::TempDir;

use common::{init_project, with_env, write_profile, write_skill};

fn write_store_skill(store: &std::path::Path, id: &str, leaf: &str) {
    let dir = store.join(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {leaf}\ndescription: Test skill {leaf} for qualified id tests.\n---\n"),
    )
    .unwrap();
}

fn import_repo_skill(home: &std::path::Path, store: &std::path::Path, repo: &str, src: &TempDir) {
    with_env(home, store)
        .args([
            "import",
            src.path().join("deploy").to_str().unwrap(),
            "--copy",
            "--repo",
            repo,
            "--as",
            "deploy",
        ])
        .assert()
        .success();
}

#[test]
fn same_deploy_skill_in_two_repos_coexists() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    for repo in ["agent-skills", "other-repo"] {
        let src = TempDir::new().unwrap();
        write_skill(src.path(), "deploy");
        import_repo_skill(home.path(), store.path(), repo, &src);
    }

    with_env(home.path(), store.path())
        .args(["ls", "-s", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"agent-skills/deploy\""))
        .stdout(predicate::str::contains("\"other-repo/deploy\""));
}

#[test]
fn profile_show_lists_qualified_ids() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    for repo in ["agent-skills", "other-repo"] {
        let src = TempDir::new().unwrap();
        write_skill(src.path(), "deploy");
        import_repo_skill(home.path(), store.path(), repo, &src);
    }

    write_profile(
        store.path(),
        "work",
        &["agent-skills/deploy", "other-repo/deploy"],
    );

    with_env(home.path(), store.path())
        .args(["profile", "show", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("agent-skills/deploy"))
        .stdout(predicate::str::contains("other-repo/deploy"));
}

#[test]
fn status_shows_store_id_for_repo_qualified_skills() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    let src = TempDir::new().unwrap();
    write_skill(src.path(), "deploy");
    import_repo_skill(home.path(), store.path(), "agent-skills", &src);

    write_profile(store.path(), "work", &["agent-skills/deploy"]);
    with_env(home.path(), store.path())
        .args(["add-profile", "work"])
        .assert()
        .success();

    with_env(home.path(), store.path())
        .args(["status", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"store_id\":\"agent-skills/deploy\"",
        ))
        .stdout(predicate::str::contains("\"name\":\"deploy\""));

    with_env(home.path(), store.path())
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("deploy"))
        .stdout(predicate::str::contains("agent-skills/deploy"));
}

#[test]
fn doctor_errors_on_unresolvable_placement_conflict() {
    let home = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    init_project(home.path(), store.path());

    for id in ["team/tdd", "other/tdd", "team__tdd"] {
        write_store_skill(store.path(), id, id.rsplit('/').next().unwrap());
    }

    write_profile(
        store.path(),
        "work",
        &["team/tdd", "other/tdd", "team__tdd"],
    );

    with_env(home.path(), store.path())
        .args(["add-profile", "work"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("resolve conflict"));

    common::set_active_profiles(home.path(), &["work"]);

    with_env(home.path(), store.path())
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("profile.resolve_conflict"))
        .stdout(predicate::str::contains("team__tdd"));

    with_env(home.path(), store.path())
        .args(["doctor", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "\"code\":\"profile.resolve_conflict\"",
        ));
}
