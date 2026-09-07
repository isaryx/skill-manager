#!/usr/bin/env bash
#
# Compare the installed skm version to the latest GitHub release.
#
# Usage:
#   scripts/check-update.sh
#   scripts/check-update.sh --json
#
# Exit codes:
#   0  up to date (or running a newer build than the latest release)
#   1  update available
#   2  usage error
#   3  could not determine current or latest version
set -euo pipefail

{ # Prevent execution if this script was only partially downloaded

GITHUB_REPO="isaryx/skill-manager"
BINARY_NAME="skm"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/github-release.sh
source "${SCRIPT_DIR}/lib/github-release.sh"
INSTALL_SCRIPT_URL="https://raw.githubusercontent.com/${GITHUB_REPO}/master/scripts/install.sh"
RELEASES_URL="https://github.com/${GITHUB_REPO}/releases"

usage() {
  cat <<'EOF'
Check whether skm is up to date with the latest GitHub release.

Usage:
  check-update.sh [options]

Options:
      --json              Emit machine-readable JSON on stdout
      --current <version> Override the running version (for tests)
      --latest <version>  Override the latest release (for tests; skips network)
  -h, --help              Show this help

Exit codes:
  0  up to date
  1  update available
  2  usage error
  3  could not determine current or latest version
EOF
}

log() {
  printf '%s\n' "$*" >&2
}

die() {
  log "error: $*"
  exit 3
}

usage_error() {
  log "error: $*"
  log "run with --help for usage"
  exit 2
}

strip_v() {
  local version="$1"
  version="${version#v}"
  [[ -n "$version" ]] || usage_error "empty version"
  printf '%s' "$version"
}

resolve_latest() {
  local version

  if version="$(resolve_latest_github_release "${GITHUB_REPO}")"; then
    printf '%s' "$version"
    return 0
  fi

  if ! command -v curl >/dev/null 2>&1 && ! command -v gh >/dev/null 2>&1; then
    die "missing required command: curl or gh"
  fi
  die "could not fetch latest release from GitHub"
}

detect_current() {
  local version

  if ! command -v "$BINARY_NAME" >/dev/null 2>&1; then
    die "${BINARY_NAME} is not on PATH"
  fi

  if ! version="$("$BINARY_NAME" --version 2>/dev/null)"; then
    die "failed to read ${BINARY_NAME} version"
  fi

  version="$(printf '%s' "$version" | awk '{print $NF}')"
  [[ -n "$version" ]] || die "could not parse ${BINARY_NAME} --version output"
  strip_v "$version"
}

version_lt() {
  local left="$1" right="$2"
  [[ "$(printf '%s\n' "$left" "$right" | sort -V | head -n1)" == "$left" && "$left" != "$right" ]]
}

print_upgrade_hints() {
  log ""
  log "Upgrade options:"
  log "  curl -fsSL ${INSTALL_SCRIPT_URL} | bash"
  log "  brew upgrade ${BINARY_NAME}   # if installed via isaryx/collection"
  log "  ${RELEASES_URL}"
}

emit_json() {
  local current="$1" latest="$2" update_available="$3"
  printf '{"current":"%s","latest":"%s","update_available":%s}\n' \
    "$current" "$latest" "$update_available"
}

main() {
  local current="" latest="" json=false update_available=false

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --json)
        json=true
        shift
        ;;
      --current)
        [[ $# -ge 2 ]] || usage_error "$1 requires a version"
        current="$(strip_v "$2")"
        shift 2
        ;;
      --latest)
        [[ $# -ge 2 ]] || usage_error "$1 requires a version"
        latest="$(strip_v "$2")"
        shift 2
        ;;
      -h | --help)
        usage
        exit 0
        ;;
      *)
        usage_error "unknown option: $1"
        ;;
    esac
  done

  if [[ -z "$current" ]]; then
    current="$(detect_current)"
  fi

  if [[ -z "$latest" ]]; then
    latest="$(resolve_latest)"
  fi

  if version_lt "$current" "$latest"; then
    update_available=true
    log "${BINARY_NAME} ${current} → latest ${latest}"
    print_upgrade_hints
  elif version_lt "$latest" "$current"; then
    log "${BINARY_NAME} ${current} (latest release is ${latest})"
  else
    log "${BINARY_NAME} ${current} is up to date"
  fi

  if [[ "$json" == true ]]; then
    emit_json "$current" "$latest" "$update_available"
  fi

  if [[ "$update_available" == true ]]; then
    exit 1
  fi
  exit 0
}

main "$@"

} # End partial-download guard
