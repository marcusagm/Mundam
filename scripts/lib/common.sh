#!/usr/bin/env bash
# Shared helpers for the macOS/Linux setup scripts. Meant to be sourced, not executed.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TAURI_DIR="$REPO_ROOT/src-tauri"
PDFIUM_VERSION="7543"

SKIP_SYSTEM=0
RUN_CHECK=0
ASSUME_YES=0
WARNINGS=()

parse_args() {
    for arg in "$@"; do
        case "$arg" in
            --skip-system) SKIP_SYSTEM=1 ;;
            --check) RUN_CHECK=1 ;;
            -y|--yes) ASSUME_YES=1 ;;
            -h|--help)
                cat <<EOF
Usage: scripts/setup.sh [options]

  --skip-system   Do not install system packages (only check, fetch and configure)
  --check         Run 'cargo check' at the end to validate the native build
  -y, --yes       Non-interactive: assume "yes" for package manager prompts
  -h, --help      Show this help
EOF
                exit 0
                ;;
            *) echo "Unknown option: $arg (use --help)" >&2; exit 2 ;;
        esac
    done
}

log()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m ✓\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m ! %s\033[0m\n' "$*"; WARNINGS+=("$*"); }
die()  { printf '\033[1;31m ✗ %s\033[0m\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

confirm() {
    [ "$ASSUME_YES" = 1 ] && return 0
    local reply
    read -r -p "$1 [Y/n] " reply
    [[ -z "$reply" || "$reply" =~ ^[Yy] ]]
}

ensure_rust() {
    log "Checking Rust toolchain"
    if ! have cargo && [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    fi
    if ! have cargo; then
        confirm "Rust (rustup) is not installed. Install it now?" || die "Rust is required."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    fi
    ok "$(rustc --version)"
}

ensure_node() {
    log "Checking Node.js"
    have node || die "Node.js (>= 18, 20 LTS recommended) is required: https://nodejs.org or use nvm/fnm."
    local major
    major="$(node -p 'process.versions.node.split(".")[0]')"
    [ "$major" -ge 18 ] || die "Node.js >= 18 is required (found $(node --version))."
    have npm || die "npm not found."
    ok "node $(node --version), npm $(npm --version)"
}

init_submodules() {
    log "Fetching git submodules (GoPro GPR SDK)"
    if [ -e "$REPO_ROOT/.git" ]; then
        git -C "$REPO_ROOT" submodule update --init --recursive
        ok "Submodules ready"
    else
        warn "Not a git checkout; make sure src-tauri/vendor/gpr_sdk is populated."
    fi
}

install_node_deps() {
    log "Installing frontend dependencies"
    (cd "$REPO_ROOT" && npm install)
    ok "npm dependencies installed"
}

# Succeeds if the file is a binary/library for the current OS (guards against files
# committed for another platform, e.g. a macOS ffmpeg on Linux).
is_native_binary() {
    local f="$1" magic
    [ -f "$f" ] || return 1
    magic="$(head -c 4 "$f" | od -An -tx1 | tr -d ' \n')"
    case "$(uname -s)" in
        Linux)  [ "$magic" = "7f454c46" ] ;;
        Darwin) case "$magic" in cffaedfe|feedfacf|cafebabe|feedface|cefaedfe) return 0 ;; *) return 1 ;; esac ;;
        *) return 1 ;;
    esac
}

# Some bundled files are tracked in git for macOS only. When replacing them on another
# platform, keep git from reporting/committing the local change.
untrack_local_change() {
    [ -e "$REPO_ROOT/.git" ] || return 0
    if git -C "$REPO_ROOT" ls-files --error-unmatch "$1" >/dev/null 2>&1; then
        git -C "$REPO_ROOT" update-index --skip-worktree "$1"
    fi
}

# Copies a system tool into src-tauri/<dir>/<name> (used for dev runs and bundling).
stage_tool() {
    local name="$1" dest_dir="$2" required="$3" src dest
    dest="$TAURI_DIR/$dest_dir/$name"
    mkdir -p "$TAURI_DIR/$dest_dir"
    if is_native_binary "$dest" && "$dest" -version >/dev/null 2>&1; then
        ok "$name already staged in src-tauri/$dest_dir"
        return 0
    fi
    src="$(command -v "$name" || true)"
    if [ -z "$src" ]; then
        [ "$required" = required ] && die "$name not found in PATH after installation."
        warn "$name not found; related features will be unavailable."
        return 0
    fi
    cp -f "$src" "$dest"
    chmod +x "$dest"
    untrack_local_change "src-tauri/$dest_dir/$name"
    ok "Staged $name from $src"
}

stage_ffmpeg() {
    local platform="$1"
    log "Staging FFmpeg / FFprobe"
    stage_tool ffmpeg "resources/ffmpeg/$platform" required
    stage_tool ffprobe "resources/ffmpeg/$platform" optional
}

# Assimp is optional (3D model conversion). The app looks for resources/assimp/<platform>/bin/assimp.
stage_assimp() {
    local platform="$1" dest
    log "Staging Assimp"
    dest="$TAURI_DIR/resources/assimp/$platform/bin/assimp"
    mkdir -p "$TAURI_DIR/resources/assimp/$platform/bin"
    if is_native_binary "$dest"; then
        ok "assimp already staged"
    elif have assimp; then
        cp -f "$(command -v assimp)" "$dest"
        chmod +x "$dest"
        ok "Staged assimp from $(command -v assimp)"
    else
        warn "assimp CLI not found; 3D model previews (OBJ/FBX/DAE...) will be unavailable."
    fi
}

# PDFium is downloaded from bblanchon/pdfium-binaries (not available in package managers).
ensure_pdfium() {
    local lib_name="$1" file="$2" lib_subdir="$3" dest tmp
    log "Checking PDFium"
    dest="$TAURI_DIR/resources/pdfium/$lib_name"
    mkdir -p "$TAURI_DIR/resources/pdfium"
    if is_native_binary "$dest"; then
        ok "PDFium already present"
        return 0
    fi
    tmp="$(mktemp -d)"
    log "Downloading PDFium ($file, chromium/$PDFIUM_VERSION)"
    curl -fL "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F${PDFIUM_VERSION}/${file}" -o "$tmp/$file" \
        || { rm -rf "$tmp"; die "Failed to download PDFium."; }
    tar -xzf "$tmp/$file" -C "$tmp"
    cp -f "$tmp/$lib_subdir/$lib_name" "$dest"
    rm -rf "$tmp"
    untrack_local_change "src-tauri/resources/pdfium/$lib_name"
    ok "PDFium installed at src-tauri/resources/pdfium/$lib_name"
}

ensure_soundfont() {
    log "Checking SoundFont (MIDI preview)"
    if ls "$TAURI_DIR"/resources/soundfonts/*.sf2 >/dev/null 2>&1; then
        ok "SoundFont present"
    else
        warn "No .sf2 in src-tauri/resources/soundfonts; MIDI preview will be unavailable."
    fi
}

finish() {
    if [ "$RUN_CHECK" = 1 ]; then
        log "Running cargo check (first run compiles LibRaw and GPR SDK; may take several minutes)"
        (cd "$TAURI_DIR" && cargo check)
    fi
    echo
    if [ "${#WARNINGS[@]}" -gt 0 ]; then
        printf '\033[1;33mSetup finished with warnings:\033[0m\n'
        for w in "${WARNINGS[@]}"; do echo "  - $w"; done
    else
        printf '\033[1;32mSetup finished successfully.\033[0m\n'
    fi
    echo
    echo "Next step:  npm run tauri dev"
}
