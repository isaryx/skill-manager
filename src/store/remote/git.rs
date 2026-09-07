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

pub fn clone_repo(url: &str, dest: &Path) -> Result<(), SkmError> {
    require_git()?;
    if dest.exists() {
        return Err(SkmError::Usage(format!(
            "checkout path already exists: {}",
            dest.display()
        )));
    }
    run_git(&["clone", "--quiet", url, dest.to_str().unwrap_or("")], None)
        .map_err(|err| clarify_clone_error(url, err))
}

pub fn pull_ff_only(checkout: &Path) -> Result<String, SkmError> {
    require_git()?;
    run_git(&["pull", "--ff-only", "--quiet"], Some(checkout))?;
    current_commit(checkout)
}

pub fn current_commit(checkout: &Path) -> Result<String, SkmError> {
    require_git()?;
    let output = git_output(&["rev-parse", "HEAD"], Some(checkout))?;
    Ok(output.trim().to_string())
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
            SkmError::GitCommandFailed { command, stderr: hint }
        }
        other => other,
    }
}

fn run_git(args: &[&str], cwd: Option<&Path>) -> Result<(), SkmError> {
    let output = Command::new("git").args(args).current_dir(cwd.unwrap_or(Path::new("."))).output()?;
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
    let output = Command::new("git").args(args).current_dir(cwd.unwrap_or(Path::new("."))).output()?;
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
