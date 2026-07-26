#!/usr/bin/env bash
# dwell — quick install script
# Usage: curl -fsSL https://github.com/grave0x/dwell/raw/main/scripts/install.sh | sh
set -euo pipefail

REPO="grave0x/dwell"
BIN_NAME="dwell-cli"

# Detect platform
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64) ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

case "$OS" in
  linux)   TARGET="${ARCH}-unknown-linux-gnu" ;;
  darwin)  TARGET="${ARCH}-apple-darwin" ;;
  *)       echo "Unsupported OS: $OS"; exit 1 ;;
esac

# Determine install dir
if [ -w /usr/local/bin ]; then
  DEST="/usr/local/bin"
elif [ -w "$HOME/.local/bin" ]; then
  DEST="$HOME/.local/bin"
elif [ -w "$HOME/bin" ]; then
  DEST="$HOME/bin"
else
  mkdir -p "$HOME/.local/bin"
  DEST="$HOME/.local/bin"
fi

echo "→ Installing dwell ($TARGET) to $DEST"

# Get latest release tag
TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
if [ -z "$TAG" ]; then
  echo "! Could not determine latest release — falling back to main branch build"
  TAG="main"
fi

# Download binary
URL="https://github.com/$REPO/releases/download/$TAG/${BIN_NAME}-${TARGET}"
echo "→ Downloading $URL"
curl -fsSL "$URL" -o "$DEST/dwell"
chmod +x "$DEST/dwell"

echo "✓ dwell installed to $DEST/dwell"

# Install shell completions
SHELL_NAME="$(basename "$SHELL" 2>/dev/null || true)"
case "$SHELL_NAME" in
  bash)
    "$DEST/dwell" completion bash | sudo tee /usr/share/bash-completion/completions/dwell >/dev/null 2>&1 && \
      echo "✓ bash completions installed" || echo "  (run 'dwell completion bash' to install manually)"
    ;;
  zsh)
    mkdir -p "$HOME/.zfunc"
    "$DEST/dwell" completion zsh > "$HOME/.zfunc/_dwell" 2>/dev/null && \
      echo "✓ zsh completions installed to ~/.zfunc/_dwell" || true
    ;;
  fish)
    mkdir -p "$HOME/.config/fish/completions"
    "$DEST/dwell" completion fish > "$HOME/.config/fish/completions/dwell.fish" 2>/dev/null && \
      echo "✓ fish completions installed" || true
    ;;
esac

echo "  Run 'dwell init' to get started."
