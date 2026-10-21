#!/usr/bin/env bash
# One-line installer for ROG Control: downloads the latest release, checks
# its sha256 and runs the bundled install.sh. No sudo needed.
#   curl -fsSL https://raw.githubusercontent.com/anilsrml/asusctl-modern-ui/main/packaging/get.sh | bash
# Arguments are passed on to install.sh, e.g. `bash -s -- --no-desktop-shortcut`.
set -euo pipefail

REPO="${ROGCC_REPO:-anilsrml/asusctl-modern-ui}"
API="https://api.github.com/repos/$REPO/releases/latest"

die() { echo "error: $*" >&2; exit 1; }

[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || die "only x86_64 Linux is supported"
for tool in curl tar sha256sum; do
    command -v "$tool" >/dev/null || die "$tool is required"
done

echo "Finding the latest release of $REPO..."
urls="$(curl -fsSL "$API" | grep -o '"browser_download_url": *"[^"]*"' | sed 's/.*"\(https[^"]*\)"/\1/')"
tarball_url="$(grep -- '-x86_64-linux\.tar\.gz$' <<<"$urls" | head -1)"
sum_url="$(grep -- '-x86_64-linux\.tar\.gz\.sha256$' <<<"$urls" | head -1)"
[[ -n "$tarball_url" && -n "$sum_url" ]] || die "no x86_64 Linux package in the latest release"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
name="$(basename "$tarball_url")"

echo "Downloading $name..."
curl -fsSL -o "$work/$name" "$tarball_url"
curl -fsSL -o "$work/$name.sha256" "$sum_url"
(cd "$work" && sha256sum -c --status "$name.sha256") || die "checksum mismatch, not installing"

tar -xzf "$work/$name" -C "$work"
bash "$work/${name%.tar.gz}/install.sh" "$@"
