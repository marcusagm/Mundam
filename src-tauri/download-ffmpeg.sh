#!/bin/bash
# Script to download FFmpeg binaries for Elleven Library
# Run this script before building for production

set -e

FFMPEG_DIR="$(dirname "$0")/ffmpeg"
mkdir -p "$FFMPEG_DIR"

echo "=== FFmpeg Download Script for Elleven Library ==="
echo ""

# Detect OS
case "$(uname -s)" in
    Darwin)
        OS="macos"
        BINARY_NAME="ffmpeg"
        echo "Detected: macOS"
        
        # Check if Homebrew FFmpeg exists
        if command -v ffmpeg &> /dev/null; then
            FFMPEG_PATH=$(which ffmpeg)
            echo "Found system FFmpeg at: $FFMPEG_PATH"
            
            # Copy to bundle directory
            cp "$FFMPEG_PATH" "$FFMPEG_DIR/$BINARY_NAME"
            chmod +x "$FFMPEG_DIR/$BINARY_NAME"
            echo "Copied FFmpeg to: $FFMPEG_DIR/$BINARY_NAME"
        else
            echo ""
            echo "FFmpeg not found. Install with:"
            echo "  brew install ffmpeg"
            echo ""
            echo "Or download static build from:"
            echo "  https://evermeet.cx/ffmpeg/"
            exit 1
        fi
        ;;
        
    MINGW*|MSYS*|CYGWIN*|Windows_NT)
        OS="windows"
        BINARY_NAME="ffmpeg.exe"
        echo "Detected: Windows"
        echo ""
        echo "Please download FFmpeg from:"
        echo "  https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
        echo ""
        echo "Extract and copy ffmpeg.exe to:"
        echo "  $FFMPEG_DIR/$BINARY_NAME"
        exit 1
        ;;
        
    Linux)
        OS="linux"
        BINARY_NAME="ffmpeg"
        echo "Detected: Linux"
        
        # Check if system FFmpeg exists
        if command -v ffmpeg &> /dev/null; then
            FFMPEG_PATH=$(which ffmpeg)
            echo "Found system FFmpeg at: $FFMPEG_PATH"
            
            # For Linux, we need a static build for portability
            echo ""
            echo "Note: System FFmpeg may not be portable."
            echo "For production, download static build from:"
            echo "  https://johnvansickle.com/ffmpeg/"
            echo ""
            
            # Copy anyway for development
            cp "$FFMPEG_PATH" "$FFMPEG_DIR/$BINARY_NAME"
            chmod +x "$FFMPEG_DIR/$BINARY_NAME"
            echo "Copied FFmpeg to: $FFMPEG_DIR/$BINARY_NAME"
        else
            echo ""
            echo "FFmpeg not found. Install with:"
            echo "  sudo apt install ffmpeg"
            echo ""
            echo "Or download static build from:"
            echo "  https://johnvansickle.com/ffmpeg/"
            exit 1
        fi
        ;;
        
    *)
        echo "Unknown OS: $(uname -s)"
        exit 1
        ;;
esac

echo ""
echo "=== FFmpeg Setup Complete ==="
echo "Binary location: $FFMPEG_DIR/$BINARY_NAME"

# Verify it works
"$FFMPEG_DIR/$BINARY_NAME" -version | head -1
