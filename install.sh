#!/usr/bin/env bash
# =============================================================================
# install.sh — One-line installer for the Yarr MCP server
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/jmagar/yarr/main/install.sh | bash
#   # or locally:
#   bash install.sh
#
# What this script does:
#   1. Detects the host OS and architecture
#   2. Downloads the pre-built binary from GitHub releases
#   3. Installs it to ~/.local/bin (no root required)
#   4. Verifies the installation with --version
#
# Requirements: curl and tar on Linux x86_64
# =============================================================================

set -euo pipefail

# ── CONFIGURATION ─────────────────────────────────────────────────────────────

REPO="${YARR_REPO:-jmagar/yarr}"

BINARY_NAME="yarr"

SERVICE_NAME="yarr-mcp"

# TEMPLATE: Set a pinned version, or leave as "latest" to always install the
#           most recent release. Pinned is safer for production automation.
VERSION="${YARR_MCP_VERSION:-latest}"

# Install directory — default is ~/.local/bin (in PATH on most modern systems)
INSTALL_DIR="${YARR_MCP_INSTALL_DIR:-${HOME}/.local/bin}"

# ── END CONFIGURATION ─────────────────────────────────────────────────────────

# Colour support
if [[ -t 1 ]]; then
  C_GREEN='\033[0;32m' C_RED='\033[0;31m' C_YELLOW='\033[0;33m' C_BOLD='\033[1m' C_RESET='\033[0m'
else
  C_GREEN='' C_RED='' C_YELLOW='' C_BOLD='' C_RESET=''
fi

info()  { printf "${C_BOLD}[INFO]${C_RESET}  %s\n" "$*"; }
warn()  { printf "${C_YELLOW}[WARN]${C_RESET}  %s\n" "$*"; }
error() { printf "${C_RED}[ERROR]${C_RESET} %s\n" "$*" >&2; }
ok()    { printf "${C_GREEN}[OK]${C_RESET}    %s\n" "$*"; }

# ── Detect OS and architecture ────────────────────────────────────────────────

detect_platform() {
  local os arch

  case "$(uname -s)" in
    Linux)  os="linux" ;;
    *)
      error "Unsupported OS: $(uname -s)"
      error "Pre-built shell installer assets are currently published for Linux x86_64 only."
      error "Build from source: cargo install --git https://github.com/${REPO}"
      exit 1
      ;;
  esac

  case "$(uname -m)" in
    x86_64|amd64) arch="x86_64" ;;
    *)
      error "Unsupported architecture: $(uname -m)"
      error "Pre-built shell installer assets are currently published for Linux x86_64 only."
      exit 1
      ;;
  esac

  PLATFORM="${os}-${arch}"
  ARCHIVE_EXT="tar.gz"
}

# ── Resolve version ───────────────────────────────────────────────────────────

resolve_version() {
  if [[ "${VERSION}" == "latest" ]]; then
    info "Resolving latest release from GitHub..."
    VERSION="$(
      curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' \
        | sed 's/.*"tag_name":[[:space:]]*"//;s/".*//'
    )"
    if [[ -z "${VERSION}" ]]; then
      error "Could not resolve latest version. Check that ${REPO} has releases."
      exit 1
    fi
    info "Latest version: ${VERSION}"
  fi
}

# ── Download and install ──────────────────────────────────────────────────────

# Set once main() starts so the EXIT trap can always reference it safely.
TMP_DIR=""

download_and_install() {
  TMP_DIR="$(mktemp -d)"
  local tmp_dir="${TMP_DIR}"

  local base_url="https://github.com/${REPO}/releases/download/${VERSION}"
  local archive
  case "${PLATFORM}" in
    linux-x86_64) archive="${BINARY_NAME}-x86_64.${ARCHIVE_EXT}" ;;
    *)
      error "Unsupported platform: ${PLATFORM}"
      error "Pre-built shell installer assets are currently published for Linux x86_64 only."
      exit 1
      ;;
  esac
  local url="${base_url}/${archive}"

  info "Downloading ${SERVICE_NAME} ${VERSION} for ${PLATFORM}..."
  info "URL: ${url}"

  if ! curl -fsSL --progress-bar "${url}" -o "${tmp_dir}/${archive}"; then
    error "Download failed: ${url}"
    error "Check that ${REPO}/releases has an asset for ${PLATFORM}"
    exit 1
  fi

  # If a release includes .sha256 files:
  # local checksum_url="${base_url}/${archive}.sha256"
  # if curl -fsSL "${checksum_url}" -o "${tmp_dir}/${archive}.sha256" 2>/dev/null; then
  #   (cd "${tmp_dir}" && sha256sum --check "${archive}.sha256")
  #   ok "Checksum verified"
  # else
  #   warn "No checksum file found at ${checksum_url} — skipping verification"
  # fi

  info "Extracting..."
  tar -xzf "${tmp_dir}/${archive}" -C "${tmp_dir}"

  local binary
  binary="$(find "${tmp_dir}" -type f -name "${BINARY_NAME}" | head -1)"
  if [[ -z "${binary}" ]]; then
    error "Binary '${BINARY_NAME}' not found in archive"
    exit 1
  fi

  mkdir -p "${INSTALL_DIR}"
  install -m 755 "${binary}" "${INSTALL_DIR}/${BINARY_NAME}"
  ok "Installed ${BINARY_NAME} to ${INSTALL_DIR}/${BINARY_NAME}"
}

# ── Verify installation ───────────────────────────────────────────────────────

verify_installation() {
  # Ensure install dir is in PATH
  if ! command -v "${BINARY_NAME}" &>/dev/null; then
    warn "${INSTALL_DIR} is not in your PATH."
    warn "Add this to your shell config (.bashrc, .zshrc, etc.):"
    warn "  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
    warn "Then run: ${BINARY_NAME} --version"
    return
  fi

  local version_output
  if ! version_output="$("${INSTALL_DIR}/${BINARY_NAME}" --version 2>&1)"; then
    error "${BINARY_NAME} --version failed after install: ${version_output}"
    exit 1
  fi
  ok "${version_output}"
  ok "${SERVICE_NAME} installed successfully"
}

# ── Setup (optional) ──────────────────────────────────────────────────────────

post_install_message() {
  printf '\n'
  printf '%b=== Next steps ===%b\n' "${C_BOLD}" "${C_RESET}"
  printf '  1. Copy the yarr config:   cp .env.example .env\n'
  printf '  2. Edit .env and set:         YARR_SERVICES plus per-service URL/key vars\n'
  printf '  3. Generate an auth token:    openssl rand -hex 32\n'
  printf '  4. Start the server:          %s serve\n' "${BINARY_NAME}"
  printf '  5. Check health:              curl http://localhost:40070/health\n'
  printf '\n'
  printf '  Or deploy with Docker:        docker compose up -d\n'
  printf '\n'
  printf '  Full docs: https://github.com/%s\n' "${REPO}"
  printf '\n'
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
  trap '[[ -n "${TMP_DIR}" ]] && rm -rf -- "${TMP_DIR}"' EXIT

  printf '%b%s%b\n' "${C_BOLD}" "$(printf '=%.0s' {1..60})" "${C_RESET}"
  printf '%b  %s Installer%b\n' "${C_BOLD}" "${SERVICE_NAME}" "${C_RESET}"
  printf '%b%s%b\n\n' "${C_BOLD}" "$(printf '=%.0s' {1..60})" "${C_RESET}"

  detect_platform
  resolve_version
  download_and_install
  verify_installation
  post_install_message
}

main "$@"
