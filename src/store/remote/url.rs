use crate::error::SkmError;

/// Resolve a user ref to a git clone URL.
///
/// - `owner/repo` → `https://github.com/owner/repo.git`
/// - `https://`, `git@`, `ssh://`, `file://` → unchanged (after trim)
/// - Absolute local path → `file://` URL
pub fn resolve_url(ref_str: &str) -> Result<String, SkmError> {
    let trimmed = ref_str.trim();
    if trimmed.is_empty() {
        return Err(SkmError::Usage("repository ref cannot be empty".into()));
    }

    if trimmed.starts_with("https://")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("git@")
        || trimmed.starts_with("ssh://")
        || trimmed.starts_with("file://")
    {
        return Ok(trimmed.to_string());
    }

    if trimmed.starts_with('/') {
        let canonical = std::path::Path::new(trimmed)
            .canonicalize()
            .map_err(|e| SkmError::Usage(format!("invalid path `{trimmed}`: {e}")))?;
        return Ok(format!("file://{}", canonical.display()));
    }

    if is_owner_repo(trimmed) {
        return Ok(format!("https://github.com/{trimmed}.git"));
    }

    Err(SkmError::Usage(format!(
        "invalid repository ref `{trimmed}`; use owner/repo, a git URL, or an absolute path"
    )))
}

/// Derive the default registry name from a ref or URL.
pub fn name_from_url(ref_str: &str) -> Result<String, SkmError> {
    let trimmed = ref_str.trim();
    if is_owner_repo(trimmed) {
        return Ok(trimmed
            .rsplit('/')
            .next()
            .unwrap_or(trimmed)
            .to_string());
    }

    let url = if trimmed.starts_with('/') {
        resolve_url(trimmed)?
    } else if trimmed.starts_with("https://")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("git@")
        || trimmed.starts_with("ssh://")
        || trimmed.starts_with("file://")
    {
        trimmed.to_string()
    } else {
        return Err(SkmError::Usage(format!(
            "cannot derive name from `{trimmed}`; pass --name"
        )));
    };

    let segment = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(&url);
    let name = segment.trim_end_matches(".git");
    if name.is_empty() {
        return Err(SkmError::Usage(
            "cannot derive repository name from URL; pass --name".into(),
        ));
    }
    Ok(name.to_string())
}

/// Normalize a URL for duplicate detection (lowercase host, strip `.git` suffix).
pub fn normalize_url(url: &str) -> String {
    let without_git = url.trim_end_matches('/').trim_end_matches(".git");

    if let Some(rest) = without_git.strip_prefix("https://") {
        return format!("https://{}", rest.to_lowercase());
    }
    if let Some(rest) = without_git.strip_prefix("http://") {
        return format!("http://{}", rest.to_lowercase());
    }
    if let Some(rest) = without_git.strip_prefix("git@") {
        return format!("git@{}", rest.to_lowercase());
    }
    if let Some(rest) = without_git.strip_prefix("ssh://") {
        return format!("ssh://{}", rest.to_lowercase());
    }
    if let Some(rest) = without_git.strip_prefix("file://") {
        return format!("file://{}", rest);
    }

    without_git.to_lowercase()
}

fn is_owner_repo(s: &str) -> bool {
    if s.contains("://") || s.starts_with('/') || s.contains('\\') {
        return false;
    }
    let parts: Vec<&str> = s.split('/').collect();
    parts.len() == 2 && parts[0].len() > 0 && parts[1].len() > 0 && !parts[0].contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_repo_resolves_to_github_https() {
        assert_eq!(
            resolve_url("jverhoeks/myskills").unwrap(),
            "https://github.com/jverhoeks/myskills.git"
        );
    }

    #[test]
    fn https_url_passes_through() {
        let url = "https://github.com/o/r.git";
        assert_eq!(resolve_url(url).unwrap(), url);
    }

    #[test]
    fn ssh_url_passes_through() {
        let url = "git@github.com:o/r.git";
        assert_eq!(resolve_url(url).unwrap(), url);
    }

    #[test]
    fn name_from_owner_repo() {
        assert_eq!(name_from_url("jverhoeks/myskills").unwrap(), "myskills");
    }

    #[test]
    fn name_from_https_url() {
        assert_eq!(
            name_from_url("https://github.com/o/my-repo.git").unwrap(),
            "my-repo"
        );
    }

    #[test]
    fn normalize_strips_git_and_lowercases_host() {
        assert_eq!(
            normalize_url("https://GitHub.com/O/R.git"),
            "https://github.com/o/r"
        );
        assert_eq!(
            normalize_url("git@github.com:O/R.git"),
            "git@github.com:o/r"
        );
    }
}
