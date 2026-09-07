#!/usr/bin/env bash
#
# Install skm from GitHub releases.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/isaryx/skill-manager/master/scripts/install.sh | bash
#   curl -fsSL ... | bash -s -- --install-dir /usr/local/bin
#
# Environment:
#   SKM_INSTALL_DIR   Install directory (default: ~/.local/bin)
#   SKM_VERSION       Pin a release tag or version (e.g. v0.3.3 or 0.3.3)
set -euo pipefail

{ # Prevent execution if this script was only partially downloaded

GITHUB_REPO="isaryx/skill-manager"
BINARY_NAME="skm"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/github-release.sh
source "${SCRIPT_DIR}/lib/github-release.sh"
DEFAULT_INSTALL_DIR="${HOME}/.local/bin"
tmpdir=""

cleanup() {
  if [[ -n "$tmpdir" && -d "$tmpdir" ]]; then
    rm -rf "$tmpdir"
  fi
}
trap cleanup EXIT

usage() {
  cat <<'EOF'
Install skm from GitHub releases.

Usage:
  install.sh [options]

Options:
  -d, --install-dir <dir>   Install directory (default: ~/.local/bin)
  -v, --version <version>   Release version (default: latest)
      --dry-run             Print the install plan without downloading or writing
      --no-verify           Skip SHA256 checksum verification
  -h, --help                Show this help

Environment:
  SKM_INSTALL_DIR           Same as --install-dir
  SKM_VERSION               Same as --version
EOF
}

log() {
  printf '%s\n' "$*" >&2
}

die() {
  log "error: $*"
  exit 1
}

usage_error() {
  log "error: $*"
  log "run with --help for usage"
  exit 2
}

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    die "missing required command: $1"
  fi
}

need_sha256_cmd() {
  if command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1; then
    return 0
  fi
  die "missing required command: sha256sum or shasum (needed to verify downloads)"
}

uname_cmd() {
  if [[ -x /usr/bin/uname ]]; then
    printf '%s' /usr/bin/uname
  else
    printf '%s' uname
  fi
}

detect_platform() {
  local os arch uname

  uname="$(uname_cmd)"
  os="$("$uname" -s)"
  arch="$("$uname" -m)"

  case "$os" in
    Darwin) platform="macos" ;;
    Linux) platform="linux" ;;
    *)
      die "unsupported operating system: ${os} (skm supports macOS and Linux only)"
      ;;
  esac

  case "$arch" in
    arm64 | aarch64) slug="${platform}-arm64" ;;
    x86_64 | amd64) slug="${platform}-x86_64" ;;
    *)
      die "unsupported architecture: ${arch} (skm supports arm64 and x86_64/amd64 only)"
      ;;
  esac
}

resolve_version() {
  local version

  if [[ -n "${version_arg:-}" ]]; then
    version="${version_arg#v}"
    printf '%s' "$version"
    return 0
  fi

  if [[ -n "${SKM_VERSION:-}" ]]; then
    version="${SKM_VERSION#v}"
    printf '%s' "$version"
    return 0
  fi

  if ! version="$(resolve_latest_github_release "${GITHUB_REPO}")"; then
    if ! command -v curl >/dev/null 2>&1 && ! command -v gh >/dev/null 2>&1; then
      die "missing required command: curl or gh"
    fi
    die "could not determine latest release version"
  fi
  printf '%s' "$version"
}

sha256_file() {
  local file="$1"

  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
  else
    die "missing required command: sha256sum or shasum (needed to verify downloads)"
  fi
}

verify_checksum() {
  local archive_path="$1" version="$2" archive_name checksums_url expected actual

  archive_name="$(basename "$archive_path")"
  checksums_url="https://github.com/${GITHUB_REPO}/releases/download/v${version}/SHA256SUMS"

  if ! expected="$(
    curl -fsSL "$checksums_url" | awk -v name="$archive_name" '$2 == name { print $1; exit }'
  )"; then
    die "failed to download checksums from ${checksums_url}"
  fi

  [[ -n "$expected" ]] || die "checksum not found in SHA256SUMS for ${archive_name}"

  actual="$(sha256_file "$archive_path")"
  if [[ "$actual" != "$expected" ]]; then
    die "checksum mismatch for ${archive_name} (expected ${expected}, got ${actual})"
  fi
}

path_hint() {
  local install_dir="$1"

  case ":${PATH}:" in
    *":${install_dir}:"*) ;;
    *)
      log "Add ${install_dir} to your PATH, for example:"
      log "  export PATH=\"${install_dir}:\$PATH\""
      ;;
  esac
}

ensure_install_dir_writable() {
  local install_dir="$1" parent

  if [[ -e "$install_dir" && ! -d "$install_dir" ]]; then
    die "install path exists and is not a directory: ${install_dir}"
  fi
  if [[ -d "$install_dir" ]]; then
    [[ -w "$install_dir" ]] || die "install directory is not writable: ${install_dir}"
    return 0
  fi

  parent="$install_dir"
  while [[ ! -d "$parent" && "$parent" != "/" ]]; do
    parent="$(dirname "$parent")"
  done
  [[ -w "$parent" ]] || die "cannot create install directory: ${install_dir}"
}

main() {
  local install_dir version archive download_url checksums_url dry_run=false verify=true

  install_dir="${SKM_INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"
  version_arg=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -d | --install-dir)
        [[ $# -ge 2 ]] || usage_error "$1 requires a directory"
        install_dir="$2"
        shift 2
        ;;
      -v | --version)
        [[ $# -ge 2 ]] || usage_error "$1 requires a version"
        version_arg="$2"
        shift 2
        ;;
      --dry-run)
        dry_run=true
        shift
        ;;
      --no-verify)
        verify=false
        shift
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

  need_cmd curl
  need_cmd tar
  if [[ "$verify" == true ]]; then
    need_sha256_cmd
  fi
  if [[ "$dry_run" == false ]]; then
    need_cmd install
    ensure_install_dir_writable "$install_dir"
  fi

  detect_platform
  version="$(resolve_version)"
  archive="${BINARY_NAME}-${version}-${slug}.tar.gz"
  download_url="https://github.com/${GITHUB_REPO}/releases/download/v${version}/${archive}"
  checksums_url="https://github.com/${GITHUB_REPO}/releases/download/v${version}/SHA256SUMS"

  log "platform=${slug} version=${version}"
  log "install_dir=${install_dir}"
  log "archive=${archive}"
  log "download_url=${download_url}"
  if [[ "$verify" == true ]]; then
    log "checksums_url=${checksums_url}"
  else
    log "checksum verification: disabled"
  fi

  if [[ "$dry_run" == true ]]; then
    log "(dry-run) would download ${archive}"
    if [[ "$verify" == true ]]; then
      log "(dry-run) would verify checksum from SHA256SUMS"
    fi
    log "(dry-run) would install ${BINARY_NAME} to ${install_dir}/${BINARY_NAME}"
    path_hint "$install_dir"
    exit 0
  fi

  tmpdir="$(mktemp -d)"

  if ! curl -fsSL "$download_url" -o "${tmpdir}/${archive}"; then
    die "failed to download ${download_url}"
  fi

  if [[ "$verify" == true ]]; then
    verify_checksum "${tmpdir}/${archive}" "$version"
    log "checksum verified"
  fi

  COPYFILE_DISABLE=1 tar -xzf "${tmpdir}/${archive}" -C "$tmpdir"
  [[ -f "${tmpdir}/${BINARY_NAME}" ]] || die "archive did not contain ${BINARY_NAME}"

  mkdir -p "$install_dir"
  install -m 755 "${tmpdir}/${BINARY_NAME}" "${install_dir}/${BINARY_NAME}"

  log "installed ${BINARY_NAME} ${version} to ${install_dir}/${BINARY_NAME}"
  path_hint "$install_dir"
}

main "$@"

} # End partial-download guard
