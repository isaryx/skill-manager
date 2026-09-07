# shellcheck shell=bash
# Shared helpers for resolving GitHub release metadata.
# Source this file; do not execute directly.

# Print the latest release version (without a leading "v") for REPO on stdout.
# Returns 1 when gh and curl cannot resolve a version.
resolve_latest_github_release() {
  local repo="${1:-${GITHUB_REPO:-}}"
  local tag

  [[ -n "$repo" ]] || return 1

  if command -v gh >/dev/null 2>&1; then
    if tag="$(gh release view --repo "${repo}" --json tagName -q .tagName 2>/dev/null)"; then
      tag="${tag#v}"
      [[ -n "$tag" ]] || return 1
      printf '%s' "$tag"
      return 0
    fi
  fi

  if ! command -v curl >/dev/null 2>&1; then
    return 1
  fi

  if ! tag="$(
    curl -fsSLI -o /dev/null -w '%{url_effective}' \
      "https://github.com/${repo}/releases/latest"
  )"; then
    return 1
  fi

  tag="${tag##*/}"
  tag="${tag#v}"
  [[ -n "$tag" ]] || return 1
  printf '%s' "$tag"
}
