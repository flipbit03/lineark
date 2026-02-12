#!/bin/sh
# Install lineark — Linear CLI for humans and LLMs
# Usage: curl -fsSL https://raw.githubusercontent.com/flipbit03/lineark/main/install.sh | sh
set -e

REPO="flipbit03/lineark"
INSTALL_DIR="${LINEARK_INSTALL_DIR:-$HOME/.local/bin}"

# Detect OS and architecture.
OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}" in
  Linux)  PLATFORM="unknown-linux-musl" ;;
  Darwin) PLATFORM="apple-darwin" ;;
  *) echo "Unsupported OS: ${OS}" >&2; exit 1 ;;
esac

case "${ARCH}" in
  x86_64|amd64)  ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *) echo "Unsupported architecture: ${ARCH}" >&2; exit 1 ;;
esac

TARGET="${ARCH}-${PLATFORM}"
ASSET="lineark-${TARGET}.tar.gz"

# macOS aarch64 doesn't have a musl variant.
if [ "${OS}" = "Darwin" ] && [ "${ARCH}" = "x86_64" ]; then
  echo "macOS x86_64 binaries are not provided. Use: cargo install lineark" >&2
  exit 1
fi

# Get latest release tag.
echo "Fetching latest release..."
TAG=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
if [ -z "${TAG}" ]; then
  echo "Failed to determine latest release" >&2
  exit 1
fi
echo "Latest release: ${TAG}"

# Download and extract.
URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"
echo "Downloading ${ASSET}..."
TMPDIR=$(mktemp -d)
trap 'rm -rf "${TMPDIR}"' EXIT

curl -fsSL "${URL}" -o "${TMPDIR}/${ASSET}"
tar xzf "${TMPDIR}/${ASSET}" -C "${TMPDIR}"

# Install.
mkdir -p "${INSTALL_DIR}"
mv "${TMPDIR}/lineark" "${INSTALL_DIR}/lineark"
chmod +x "${INSTALL_DIR}/lineark"

echo "Installed lineark ${TAG} to ${INSTALL_DIR}/lineark"

# Check if install dir is in PATH.
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *) echo "Add ${INSTALL_DIR} to your PATH:"; echo "  export PATH=\"${INSTALL_DIR}:\$PATH\"" ;;
esac
