#!/bin/sh
# Install the latest `ages` release for this machine.
#
#   curl -fsSL https://raw.githubusercontent.com/eminaktas/ages/main/install.sh | sh
#
# Options (environment variables):
#   AGES_VERSION   install this version instead of the latest release (e.g. 0.1.0)
#   AGES_INSTALL   directory to install into (default: /usr/local/bin if writable, else ~/.local/bin)
#   AGES_BASE_URL  override the download base (used for testing)
set -eu

repo="eminaktas/ages"
base="${AGES_BASE_URL:-https://github.com/$repo/releases}"

os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Darwin) os="macos" ;;
  Linux) os="linux" ;;
  *) echo "ages: unsupported OS: $os (macOS and Linux only)" >&2; exit 1 ;;
esac
case "$arch" in
  x86_64|amd64) arch="x86_64" ;;
  arm64|aarch64) arch="arm64" ;;
  *) echo "ages: unsupported architecture: $arch" >&2; exit 1 ;;
esac

asset="ages-$os-$arch.tar.gz"
if [ -n "${AGES_VERSION:-}" ]; then
  url="$base/download/$AGES_VERSION/$asset"
  sums_url="$base/download/$AGES_VERSION/SHA256SUMS"
else
  url="$base/latest/download/$asset"
  sums_url="$base/latest/download/SHA256SUMS"
fi

if [ -n "${AGES_INSTALL:-}" ]; then
  dest="$AGES_INSTALL"
elif [ -w /usr/local/bin ]; then
  dest="/usr/local/bin"
else
  dest="$HOME/.local/bin"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading $asset ..."
curl -fsSL "$url" -o "$tmp/$asset"
curl -fsSL "$sums_url" -o "$tmp/SHA256SUMS"

echo "Verifying checksum ..."
expected="$(grep " $asset\$" "$tmp/SHA256SUMS" | cut -d' ' -f1)"
if [ -z "$expected" ]; then
  echo "ages: $asset not listed in SHA256SUMS" >&2; exit 1
fi
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$tmp/$asset" | cut -d' ' -f1)"
else
  actual="$(shasum -a 256 "$tmp/$asset" | cut -d' ' -f1)"
fi
if [ "$expected" != "$actual" ]; then
  echo "ages: checksum mismatch for $asset" >&2
  echo "  expected $expected" >&2
  echo "  actual   $actual" >&2
  exit 1
fi

tar -xzf "$tmp/$asset" -C "$tmp"
mkdir -p "$dest"
install -m 755 "$tmp/ages" "$dest/ages"
echo "Installed $("$dest/ages" --version) to $dest/ages"

case ":$PATH:" in
  *":$dest:"*) ;;
  *) echo "Note: $dest is not on your PATH. Add it, e.g.: export PATH=\"$dest:\$PATH\"" ;;
esac
