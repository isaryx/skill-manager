mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::TempDir;

use common::*;

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
