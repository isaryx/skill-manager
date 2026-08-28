use std::io::{self, Write};

use dialoguer::console::style;

use crate::color::color_stderr;
use crate::config::home_dir;

pub fn step(msg: impl AsRef<str>) {
    let msg = msg.as_ref();
    let mut out = io::stderr().lock();
    if color_stderr() {
        let _ = writeln!(out, "{}", style(format!("  {msg}")).dim());
    } else {
        let _ = writeln!(out, "  {msg}");
    }
}

pub fn added(skill: &str) {
    step(format!("added {skill}"));
}

pub fn wired(skill: &str, dry_run: bool) {
    if dry_run {
        step(format!("(dry-run) + {skill}"));
    } else {
        diff_line('+', skill, true);
    }
}

pub fn unwired(skill: &str, dry_run: bool) {
    if dry_run {
        step(format!("(dry-run) - {skill}"));
    } else {
        diff_line('-', skill, false);
    }
}

pub fn skipped_conflict(skill: &str, dry_run: bool) {
    if dry_run {
        step(format!("(dry-run) skipped {skill} (conflicted)"));
    } else {
        step(format!("skipped {skill} (conflicted)"));
    }
}

fn diff_line(sign: char, skill: &str, added: bool) {
    let line = format!("  {sign} {skill}");
    let mut out = io::stderr().lock();
    if color_stderr() {
        let styled = if added {
            style(&line).green()
        } else {
            style(&line).red()
        };
        let _ = writeln!(out, "{styled}");
    } else {
        let _ = writeln!(out, "{line}");
    }
}

pub fn display_path(path: &std::path::Path) -> String {
    let home = home_dir();
    if let Ok(rel) = path.strip_prefix(&home) {
        return format!("~/{}", rel.to_string_lossy());
    }
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_path_shortens_home() {
        let home = home_dir();
        let path = home.join(".skill-store/local/demo");
        assert_eq!(display_path(&path), "~/.skill-store/local/demo");
    }
}
