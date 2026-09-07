use std::path::Path;
use std::process::Command;

use crate::error::SkmError;

pub fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

pub fn require_git() -> Result<(), SkmError> {
    if git_available() {
        Ok(())
    } else {
        Err(SkmError::GitNotFound)
    }
}

pub fn clone_repo(url: &str, dest: &Path, pin: Option<&str>) -> Result<(), SkmError> {
    require_git()?;
    if dest.exists() {
        return Err(SkmError::Usage(format!(
            "checkout path already exists: {}",
            dest.display()
        )));
    }

    let dest_str = dest.to_str().unwrap_or("");
    let mut args = vec!["clone", "--quiet"];
    if let Some(pin) = pin {
        args.push("--branch");
        args.push(pin);
    }
    args.push(url);
    args.push(dest_str);

    run_git(&args, None).map_err(|err| clarify_clone_error(url, err))
}

/// Fast-forward the checkout to the latest upstream for its pin (or default branch).
pub fn update_checkout(checkout: &Path, pin: Option<&str>) -> Result<String, SkmError> {
    require_git()?;
    match pin {
        None => run_git(&["pull", "--ff-only", "--quiet"], Some(checkout))?,
        Some(pin) => {
            run_git(&["fetch", "--quiet", "origin"], Some(checkout))?;
            run_git(&["checkout", "--quiet", pin], Some(checkout))?;
            if !looks_like_commit(pin) {
                let _ = run_git(
                    &["pull", "--ff-only", "--quiet", "origin", pin],
                    Some(checkout),
                );
            }
        }
    }
    current_commit(checkout)
}

pub fn checkout_default_branch(checkout: &Path) -> Result<(), SkmError> {
    require_git()?;
    let _ = run_git(&["fetch", "--quiet", "origin"], Some(checkout));

    if let Ok(sym) = git_output(
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
        Some(checkout),
    ) {
        let branch = sym.trim().strip_prefix("origin/").unwrap_or(sym.trim());
        run_git(&["checkout", "--quiet", branch], Some(checkout))?;
        let _ = run_git(
            &["pull", "--ff-only", "--quiet", "origin", branch],
            Some(checkout),
        );
        return Ok(());
    }

    for candidate in ["main", "master"] {
        if run_git(&["checkout", "--quiet", candidate], Some(checkout)).is_ok() {
            let _ = run_git(&["pull", "--ff-only", "--quiet"], Some(checkout));
            return Ok(());
        }
    }

    Err(SkmError::Usage(
        "cannot determine default branch for remote checkout".into(),
    ))
}

pub fn checkout_ref(checkout: &Path, pin: &str) -> Result<(), SkmError> {
    require_git()?;
    run_git(&["fetch", "--quiet", "origin"], Some(checkout))?;
    run_git(&["checkout", "--quiet", pin], Some(checkout))
}

pub fn pull_ff_only(checkout: &Path) -> Result<String, SkmError> {
    update_checkout(checkout, None)
}

pub fn current_commit(checkout: &Path) -> Result<String, SkmError> {
    require_git()?;
    let output = git_output(&["rev-parse", "HEAD"], Some(checkout))?;
    Ok(output.trim().to_string())
}

/// Last commit that touched `path` within `checkout`, or `HEAD` when the path has no history.
pub fn path_last_commit(checkout: &Path, path: &Path) -> Result<String, SkmError> {
    require_git()?;
    let rel = path
        .strip_prefix(checkout)
        .map_err(|e| SkmError::Io(std::io::Error::other(e.to_string())))?;
    let rel_arg = if rel.as_os_str().is_empty() {
        ".".to_string()
    } else {
        rel.to_string_lossy().into_owned()
    };
    let output = git_output(
        &["log", "-1", "--format=%H", "--", &rel_arg],
        Some(checkout),
    )?;
    let trimmed = output.trim();
    if trimmed.is_empty() {
        current_commit(checkout)
    } else {
        Ok(trimmed.to_string())
    }
}

fn looks_like_commit(pin: &str) -> bool {
    let pin = pin.trim();
    (7..=40).contains(&pin.len()) && pin.chars().all(|c| c.is_ascii_hexdigit())
}

fn clarify_clone_error(url: &str, err: SkmError) -> SkmError {
    match err {
        SkmError::GitCommandFailed { command, stderr } => {
            let lower = stderr.to_lowercase();
            let hint = if lower.contains("repository not found") {
                format!("repository not found or not accessible: {url}")
            } else if lower.contains("could not read username")
                || lower.contains("authentication failed")
            {
                format!("cannot access repository (not found, private, or auth required): {url}")
            } else {
                stderr
            };
            SkmError::GitCommandFailed {
                command,
                stderr: hint,
            }
        }
        other => other,
    }
}

fn run_git(args: &[&str], cwd: Option<&Path>) -> Result<(), SkmError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd.unwrap_or(Path::new(".")))
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let cmd = format!("git {}", args.join(" "));
        Err(SkmError::GitCommandFailed {
            command: cmd,
            stderr,
        })
    }
}

fn git_output(args: &[&str], cwd: Option<&Path>) -> Result<String, SkmError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd.unwrap_or(Path::new(".")))
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let cmd = format!("git {}", args.join(" "));
        Err(SkmError::GitCommandFailed {
            command: cmd,
            stderr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_pin_detection() {
        assert!(looks_like_commit("abc1234"));
        assert!(looks_like_commit(
            "f3bbc1d3b82cdf56d0e793271c0a7ea064598b15"
        ));
        assert!(!looks_like_commit("main"));
        assert!(!looks_like_commit("v1.0.0"));
    }
}
