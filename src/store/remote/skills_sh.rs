//! Fetch and parse the [skills.sh](https://skills.sh) leaderboard.
//!
//! Tries `GET /api/skills` (JSON array of skills). When that is unavailable, scrapes the
//! homepage HTML — rows link to `/owner/repo/skill` with `aria-label="Weekly installs: N, …"`.

use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;

use crate::error::SkmError;
use crate::store::remote::url::{github_owner_repo, is_github_owner_repo};

const API_URL: &str = "https://skills.sh/api/skills";
const HOME_URL: &str = "https://skills.sh/";

/// One repository on the skills.sh leaderboard (aggregated from skill rows).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaderboardRepo {
    pub owner: String,
    pub repo: String,
    pub skill_count: u32,
    pub installs: u64,
}

impl LeaderboardRepo {
    pub fn owner_repo(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

/// Fetch the leaderboard, preferring the JSON API and falling back to HTML scraping.
pub fn fetch_leaderboard() -> Result<Vec<LeaderboardRepo>, SkmError> {
    if let Ok(repos) = try_fetch_api() {
        if !repos.is_empty() {
            return Ok(repos);
        }
    }
    try_fetch_html()
}

fn try_fetch_api() -> Result<Vec<LeaderboardRepo>, SkmError> {
    let body = fetch_text(API_URL)?;
    parse_api_json(&body)
}

fn try_fetch_html() -> Result<Vec<LeaderboardRepo>, SkmError> {
    let body = fetch_text(HOME_URL)?;
    let repos = parse_html(&body)?;
    if repos.is_empty() {
        return Err(fetch_failed("could not find any repositories on skills.sh"));
    }
    Ok(repos)
}

fn fetch_text(url: &str) -> Result<String, SkmError> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(60))
        .build();
    match agent.get(url).call() {
        Ok(response) => response.into_string().map_err(|e| network_error(url, e)),
        Err(ureq::Error::Status(code, _)) => {
            Err(fetch_failed(format!("{url} returned HTTP {code}")))
        }
        Err(err) => Err(network_error(url, err)),
    }
}

fn network_error(url: &str, err: impl std::fmt::Display) -> SkmError {
    fetch_failed(format!("failed to fetch {url}: {err}"))
}

fn fetch_failed(message: impl Into<String>) -> SkmError {
    SkmError::SkillsShFetchFailed {
        message: message.into(),
    }
}

#[derive(Debug, Deserialize)]
struct ApiSkill {
    owner: String,
    repo: String,
    installs: u64,
}

/// Parse the skills.sh JSON API response.
pub fn parse_api_json(body: &str) -> Result<Vec<LeaderboardRepo>, SkmError> {
    let skills: Vec<ApiSkill> = serde_json::from_str(body)
        .map_err(|e| fetch_failed(format!("invalid skills.sh API response: {e}")))?;
    Ok(aggregate_skills(
        skills
            .into_iter()
            .map(|s| (s.owner, s.repo, s.installs))
            .collect(),
    ))
}

/// Parse the skills.sh homepage HTML.
pub fn parse_html(html: &str) -> Result<Vec<LeaderboardRepo>, SkmError> {
    let mut by_repo: HashMap<(String, String), (u32, u64)> = HashMap::new();

    for segment in html.split("href=\"/") {
        if let Some((owner, repo, installs)) = parse_skill_href_segment(segment) {
            let entry = by_repo.entry((owner, repo)).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += installs;
        }
    }

    if by_repo.is_empty() {
        merge_github_link_rows(html, &mut by_repo);
    }

    Ok(sort_repos(
        by_repo
            .into_iter()
            .map(|((owner, repo), (skill_count, installs))| LeaderboardRepo {
                owner,
                repo,
                skill_count,
                installs,
            })
            .collect(),
    ))
}

fn parse_skill_href_segment(segment: &str) -> Option<(String, String, u64)> {
    let end = segment.find('"')?;
    let path = &segment[..end];
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() != 3 {
        return None;
    }
    let owner = parts[0];
    let repo = parts[1];
    if should_skip_owner(owner) || !is_github_owner_repo(owner, repo) {
        return None;
    }
    let installs = parse_weekly_installs(segment).unwrap_or(0);
    Some((owner.to_string(), repo.to_string(), installs))
}

fn merge_github_link_rows(html: &str, by_repo: &mut HashMap<(String, String), (u32, u64)>) {
    let mut search = 0;
    while let Some(idx) = html[search..].find("github.com/") {
        let start = search + idx + "github.com/".len();
        search = start;
        let rest = &html[start..];
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.')
            .unwrap_or(rest.len());
        let slug = &rest[..end];
        let parts: Vec<&str> = slug.split('/').collect();
        if parts.len() < 2 {
            continue;
        }
        let owner = parts[0];
        let repo = parts[1].trim_end_matches(".git");
        if should_skip_owner(owner) || !is_github_owner_repo(owner, repo) {
            continue;
        }
        let entry = by_repo
            .entry((owner.to_string(), repo.to_string()))
            .or_insert((0, 0));
        entry.0 += 1;
    }
}

fn parse_weekly_installs(segment: &str) -> Option<u64> {
    let marker = "Weekly installs: ";
    let start = segment.find(marker)?;
    let rest = &segment[start + marker.len()..];
    let digits: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ',')
        .collect();
    let number = digits.replace(',', "");
    number.parse().ok()
}

fn should_skip_owner(owner: &str) -> bool {
    matches!(
        owner,
        "agent" | "agents" | "topic" | "site" | "skills" | "api" | "docs" | "immutable"
    )
}

fn aggregate_skills(skills: Vec<(String, String, u64)>) -> Vec<LeaderboardRepo> {
    let mut by_repo: HashMap<(String, String), (u32, u64)> = HashMap::new();
    for (owner, repo, installs) in skills {
        if should_skip_owner(&owner) || !is_github_owner_repo(&owner, &repo) {
            continue;
        }
        let entry = by_repo.entry((owner, repo)).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += installs;
    }
    sort_repos(
        by_repo
            .into_iter()
            .map(|((owner, repo), (skill_count, installs))| LeaderboardRepo {
                owner,
                repo,
                skill_count,
                installs,
            })
            .collect(),
    )
}

fn sort_repos(mut repos: Vec<LeaderboardRepo>) -> Vec<LeaderboardRepo> {
    repos.sort_by(|a, b| {
        b.installs
            .cmp(&a.installs)
            .then_with(|| a.owner_repo().cmp(&b.owner_repo()))
    });
    repos
}

/// Format install counts for the browse list (`115k`, `1.2M`).
pub fn format_installs(installs: u64) -> String {
    if installs >= 1_000_000 {
        let millions = installs as f64 / 1_000_000.0;
        if millions >= 10.0 {
            format!("{:.0}M", millions)
        } else {
            format!("{:.1}M", millions)
        }
    } else if installs >= 1_000 {
        let thousands = installs as f64 / 1_000.0;
        if thousands >= 100.0 {
            format!("{:.0}k", thousands)
        } else {
            format!("{:.1}k", thousands)
        }
    } else {
        installs.to_string()
    }
}

/// Collect `owner/repo` keys for already registered GitHub remotes.
pub fn registered_github_owner_repos(
    repos: &[crate::config::RepoRegistration],
) -> std::collections::HashSet<String> {
    repos
        .iter()
        .filter_map(|reg| github_owner_repo(&reg.url))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_HTML: &str = r#"
<a href="/mattpocock/skills/grill-me"><svg aria-label="Weekly installs: 115,475, 107,969"></svg></a>
<a href="/mattpocock/skills/tdd"><svg aria-label="Weekly installs: 69,550, 67,971"></svg></a>
<a href="/vercel-labs/skills/find-skills"><svg aria-label="Weekly installs: 10,000, 9,000"></svg></a>
<a href="/agent/cursor/foo"><svg aria-label="Weekly installs: 999,999"></svg></a>
"#;

    #[test]
    fn parse_api_json_aggregates_by_repo() {
        let body = r#"[
            {"owner":"o","repo":"r","name":"a","installs":10},
            {"owner":"o","repo":"r","name":"b","installs":5},
            {"owner":"x","repo":"y","name":"c","installs":100}
        ]"#;
        let repos = parse_api_json(body).unwrap();
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].owner_repo(), "x/y");
        assert_eq!(repos[0].installs, 100);
        assert_eq!(repos[1].owner_repo(), "o/r");
        assert_eq!(repos[1].skill_count, 2);
        assert_eq!(repos[1].installs, 15);
    }

    #[test]
    fn parse_html_extracts_repos_and_installs() {
        let repos = parse_html(FIXTURE_HTML).unwrap();
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].owner_repo(), "mattpocock/skills");
        assert_eq!(repos[0].skill_count, 2);
        assert_eq!(repos[0].installs, 115_475 + 69_550);
        assert_eq!(repos[1].owner_repo(), "vercel-labs/skills");
        assert_eq!(repos[1].installs, 10_000);
    }

    #[test]
    fn parse_html_keeps_rows_without_install_aria_label() {
        let html = r#"<a href="/vercel-labs/skills/find-skills"></a>"#;
        let repos = parse_html(html).unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].owner_repo(), "vercel-labs/skills");
        assert_eq!(repos[0].installs, 0);
    }

    #[test]
    fn parse_html_skips_agent_paths() {
        let html = r#"<a href="/agent/cursor/foo" aria-label="Weekly installs: 1,000"></a>"#;
        assert!(parse_html(html).unwrap().is_empty());
    }

    #[test]
    fn format_installs_scales() {
        assert_eq!(format_installs(500), "500");
        assert_eq!(format_installs(12_500), "12.5k");
        assert_eq!(format_installs(115_475), "115k");
        assert_eq!(format_installs(1_250_000), "1.2M");
    }

    #[test]
    #[ignore = "manual: requires network access to skills.sh"]
    fn live_fetch_skills_sh_leaderboard() {
        let repos = fetch_leaderboard().expect("fetch");
        assert!(!repos.is_empty(), "expected repos from skills.sh");
        eprintln!("live fetch: {} repos", repos.len());
        for entry in repos.iter().take(5) {
            eprintln!(
                "  {} ({} skills, {} installs)",
                entry.owner_repo(),
                entry.skill_count,
                entry.installs
            );
        }
    }

    #[test]
    fn registered_github_owner_repos_from_urls() {
        use crate::config::RepoRegistration;
        let repos = vec![RepoRegistration {
            version: 1,
            name: "myskills".into(),
            url: "https://github.com/jverhoeks/myskills.git".into(),
            checkout: "remotes/myskills".into(),
            commit: "abc".into(),
            skills_root: "skills".into(),
            cloned_at: "".into(),
            updated_at: "".into(),
            last_pull_error: None,
            pin: None,
        }];
        let keys = registered_github_owner_repos(&repos);
        assert!(keys.contains("jverhoeks/myskills"));
    }
}
