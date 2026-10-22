#!/usr/bin/env bash
# Entry point: detects the OS and runs the matching setup script.
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "$(uname -s)" in
    Darwin) exec bash "$DIR/setup-macos.sh" "$@" ;;
    Linux)  exec bash "$DIR/setup-linux.sh" "$@" ;;
    MINGW*|MSYS*|CYGWIN*)
        exec powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(cygpath -w "$DIR/setup-windows.ps1")" "$@" ;;
    *) echo "Unsupported OS: $(uname -s)" >&2; exit 1 ;;
esac
