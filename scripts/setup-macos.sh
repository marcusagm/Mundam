#!/usr/bin/env bash
# Development environment setup for macOS.
set -euo pipefail
# shellcheck source=lib/common.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
parse_args "$@"

if [ "$SKIP_SYSTEM" = 0 ]; then
    log "Checking Xcode Command Line Tools (native compiler and SDK headers)"
    if ! xcode-select -p >/dev/null 2>&1; then
        xcode-select --install || true
        die "Finish installing the Xcode Command Line Tools, then re-run this script."
    fi
    ok "Xcode CLT found"

    if ! have brew; then
        for p in /opt/homebrew/bin/brew /usr/local/bin/brew; do
            if [ -x "$p" ]; then eval "$("$p" shellenv)"; break; fi
        done
    fi
    have brew || die "Homebrew is required: https://brew.sh (then re-run this script)."

    log "Installing system packages with Homebrew (pkg-config, ffmpeg, assimp)"
    for pkg in pkg-config ffmpeg assimp; do
        if brew list --formula "$pkg" >/dev/null 2>&1; then
            ok "$pkg already installed"
        else
            brew install "$pkg"
        fi
    done
fi

ensure_node
ensure_rust
install_node_deps

stage_ffmpeg macos
stage_assimp macos

ensure_pdfium libpdfium.dylib pdfium-mac-univ.tgz lib
ensure_soundfont

finish
