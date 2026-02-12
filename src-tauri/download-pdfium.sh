#!/bin/bash
# Script to download PDFium binaries for Mundam
# Downloads the appropriate dynamic library for the current platform

set -e

PDFIUM_DIR="$(dirname "$0")/binaries/pdfium"
mkdir -p "$PDFIUM_DIR"

VERSION="7543" # Recommended version from pdfium-render docs
BASE_URL="https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F${VERSION}"

echo "=== PDFium Download Script for Mundam ==="
echo "Version: ${VERSION}"

OS_TYPE=$(uname -s)
ARCH=$(uname -m)

case "${OS_TYPE}" in
    Darwin)
        if [ "${ARCH}" == "arm64" ]; then
            FILE="pdfium-mac-arm64.tgz"
        else
            FILE="pdfium-mac-x64.tgz"
        fi
        LIB_NAME="libpdfium.dylib"
        ;;
    Linux)
        if [ "${ARCH}" == "x86_64" ]; then
            FILE="pdfium-linux-x64.tgz"
        else
            FILE="pdfium-linux-arm64.tgz"
        fi
        LIB_NAME="libpdfium.so"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        FILE="pdfium-win-x64.zip"
        LIB_NAME="pdfium.dll"
        ;;
    *)
        echo "Unsupported OS: ${OS_TYPE}"
        exit 1
        ;;
esac

TEMP_DIR=$(mktemp -d)
echo "Downloading ${FILE}..."
curl -L "${BASE_URL}/${FILE}" -o "${TEMP_DIR}/${FILE}"

echo "Extracting..."
if [[ "${FILE}" == *.tgz ]]; then
    tar -xzf "${TEMP_DIR}/${FILE}" -C "${TEMP_DIR}"
else
    unzip -q "${TEMP_DIR}/${FILE}" -d "${TEMP_DIR}"
fi

# Find the library file in the extracted content
# The structure usually has bin/ or lib/ folders
find "${TEMP_DIR}" -name "${LIB_NAME}" -exec cp {} "${PDFIUM_DIR}/" \;

echo "Clearing temp files..."
rm -rf "${TEMP_DIR}"

if [ -f "${PDFIUM_DIR}/${LIB_NAME}" ]; then
    echo "=== PDFium Setup Complete ==="
    echo "Location: ${PDFIUM_DIR}/${LIB_NAME}"
else
    echo "Error: Failed to find ${LIB_NAME} in the downloaded archive."
    exit 1
fi
