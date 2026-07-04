#!/usr/bin/env bash
set -euo pipefail

REPO="${YARR_REPO:-jmagar/rustarr-mcp}"
INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
VERSION="${YARR_VERSION:-latest}"

usage() {
  cat <<'USAGE'
Install rustarr/yarr from GitHub Releases.

Environment:
  INSTALL_DIR  Destination directory (default: ~/.local/bin)
  YARR_VERSION Release tag such as v0.4.0 (default: latest)
  YARR_REPO    GitHub repo owner/name (default: jmagar/rustarr-mcp)
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

need() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'error: %s is required\n' "$1" >&2
    exit 1
  }
}

target_asset() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"

  case "${os}:${arch}" in
    linux:x86_64|linux:amd64)
      printf 'rustarr-x86_64.tar.gz'
      ;;
    mingw*:x86_64|msys*:x86_64|cygwin*:x86_64)
      printf 'rustarr-windows-x86_64.tar.gz'
      ;;
    *)
      printf 'error: unsupported platform %s/%s\n' "${os}" "${arch}" >&2
      exit 1
      ;;
  esac
}

need curl
need tar
need mktemp

asset="$(target_asset)"
tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT

if [[ "${VERSION}" == "latest" ]]; then
  url="https://github.com/${REPO}/releases/latest/download/${asset}"
else
  url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
fi

mkdir -p "${INSTALL_DIR}"
if [[ ! -w "${INSTALL_DIR}" ]]; then
  printf 'error: install dir is not writable: %s\n' "${INSTALL_DIR}" >&2
  exit 1
fi

printf 'Downloading %s\n' "${url}" >&2
curl -fsSL "${url}" -o "${tmpdir}/${asset}"
tar -xzf "${tmpdir}/${asset}" -C "${tmpdir}"

binary="${tmpdir}/rustarr"
if [[ ! -f "${binary}" && -f "${tmpdir}/rustarr.exe" ]]; then
  binary="${tmpdir}/rustarr.exe"
fi
if [[ ! -f "${binary}" ]]; then
  printf 'error: archive did not contain rustarr binary\n' >&2
  exit 1
fi

install -m 755 "${binary}" "${INSTALL_DIR}/rustarr"
ln -sf rustarr "${INSTALL_DIR}/yarr"

printf 'Installed rustarr to %s/rustarr\n' "${INSTALL_DIR}"
printf 'Installed yarr shim to %s/yarr\n' "${INSTALL_DIR}"
printf 'Run: yarr --version\n'
