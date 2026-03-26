#!/bin/sh
# ownrs installer — https://github.com/raiderrobert/ownrs
# Usage: curl -sSL https://raw.githubusercontent.com/raiderrobert/ownrs/main/install.sh | sh
set -e

REPO="raiderrobert/ownrs"
INSTALL_DIR="${OWNRS_HOME:-$HOME/.ownrs/bin}"

main() {
    platform="$(detect_platform)"
    arch="$(detect_arch)"
    asset="$(asset_name "$platform" "$arch")"

    if [ -z "$asset" ]; then
        echo "Error: unsupported platform/architecture: ${platform}/${arch}" >&2
        echo "Pre-built binaries are available for:" >&2
        echo "  - macOS (Apple Silicon / aarch64)" >&2
        exit 1
    fi

    url="https://github.com/${REPO}/releases/latest/download/${asset}"

    echo "Installing ownrs — ownership reconciliation CLI"
    echo ""
    echo "Detected: ${platform}/${arch}"
    echo "Downloading: ${url}"

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    if command -v curl > /dev/null 2>&1; then
        curl -fsSL "$url" -o "${tmpdir}/${asset}"
    elif command -v wget > /dev/null 2>&1; then
        wget -qO "${tmpdir}/${asset}" "$url"
    else
        echo "Error: curl or wget is required" >&2
        exit 1
    fi

    tar xzf "${tmpdir}/${asset}" -C "$tmpdir"

    mkdir -p "$INSTALL_DIR"
    cp "${tmpdir}/ownrs" "$INSTALL_DIR/ownrs"
    chmod +x "$INSTALL_DIR/ownrs"

    xattr -cr "$INSTALL_DIR/ownrs" 2>/dev/null || true
    codesign -s - "$INSTALL_DIR/ownrs" 2>/dev/null || true

    echo ""
    echo "ownrs installed to $INSTALL_DIR/ownrs"
    echo ""

    if ! echo ":$PATH:" | grep -q ":$INSTALL_DIR:"; then
        echo "Add to your PATH (add this to your shell profile):"
        echo ""
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        echo ""
    fi

    if command -v gh > /dev/null 2>&1 && gh auth status > /dev/null 2>&1; then
        echo "GitHub auth: detected via gh CLI — you're all set!"
    elif command -v gh > /dev/null 2>&1; then
        echo "GitHub auth: gh CLI found but not logged in. Run:"
        echo ""
        echo "  gh auth login"
    else
        echo "GitHub auth: set a token with read:org and repo scopes:"
        echo ""
        echo "  export GITHUB_TOKEN=<your-token>"
        echo ""
        echo "Or install the GitHub CLI (https://cli.github.com) and run: gh auth login"
    fi

    echo ""
    echo "Try it out:"
    echo ""
    echo "  ownrs org my-org --detail"
    echo ""
}

detect_platform() {
    case "$(uname -s)" in
        Darwin*) echo "macos" ;;
        *)       echo "unknown" ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        arm64|aarch64) echo "aarch64" ;;
        *)             echo "unknown" ;;
    esac
}

asset_name() {
    platform="$1"
    arch="$2"

    case "${arch}-${platform}" in
        aarch64-macos) echo "ownrs-aarch64-macos.tar.gz" ;;
        *)             echo "" ;;
    esac
}

main
