#!/usr/bin/env bash
# Development environment setup for Linux (apt, dnf, pacman and zypper supported).
set -euo pipefail
# shellcheck source=lib/common.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
parse_args "$@"

SUDO=""
[ "$(id -u)" -ne 0 ] && SUDO="sudo"

install_system_packages() {
    local yes_flag="" noconfirm=""
    if [ "$ASSUME_YES" = 1 ]; then yes_flag="-y"; noconfirm="--noconfirm"; fi

    if have apt-get; then
        log "Installing system packages with apt"
        $SUDO apt-get update
        # libwebkit2gtk-4.1-dev and friends are the Tauri v2 prerequisites.
        $SUDO apt-get install $yes_flag build-essential curl wget file git unzip pkg-config cmake \
            clang libclang-dev libssl-dev libxdo-dev patchelf \
            libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
            ffmpeg assimp-utils
    elif have dnf; then
        log "Installing system packages with dnf"
        $SUDO dnf install $yes_flag webkit2gtk4.1-devel openssl-devel curl wget file git unzip pkgconf-pkg-config cmake \
            gcc gcc-c++ make clang clang-devel libxdo-devel patchelf \
            libappindicator-gtk3-devel librsvg2-devel
        # ffmpeg/assimp may require RPM Fusion/EPEL; do not abort the whole setup.
        $SUDO dnf install $yes_flag ffmpeg-free assimp || warn "Could not install ffmpeg/assimp via dnf (enable RPM Fusion for full ffmpeg)."
    elif have pacman; then
        log "Installing system packages with pacman"
        $SUDO pacman -S --needed $noconfirm base-devel curl wget file git unzip pkgconf cmake clang \
            openssl xdotool patchelf webkit2gtk-4.1 libappindicator-gtk3 librsvg ffmpeg assimp
    elif have zypper; then
        log "Installing system packages with zypper"
        $SUDO zypper install $yes_flag -t pattern devel_basis
        $SUDO zypper install $yes_flag curl wget file git unzip pkg-config cmake clang libclang-devel \
            libopenssl-devel xdotool-devel patchelf webkit2gtk3-devel libappindicator3-devel librsvg-devel \
            ffmpeg assimp
    else
        warn "Unsupported package manager. Install manually: C/C++ toolchain, cmake, clang/libclang, pkg-config, webkit2gtk-4.1, librsvg, openssl, ffmpeg, assimp."
    fi
}

if [ "$SKIP_SYSTEM" = 0 ]; then install_system_packages; fi

ensure_node
ensure_rust
init_submodules
install_node_deps

stage_ffmpeg
stage_assimp linux

case "$(uname -m)" in
    x86_64) pdfium_file="pdfium-linux-x64.tgz" ;;
    aarch64|arm64) pdfium_file="pdfium-linux-arm64.tgz" ;;
    *) die "Unsupported architecture: $(uname -m)" ;;
esac
ensure_pdfium libpdfium.so "$pdfium_file" lib
ensure_soundfont

finish
