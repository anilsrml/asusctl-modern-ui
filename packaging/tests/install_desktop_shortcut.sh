#!/usr/bin/env bash
# install.sh puts a launcher on the user's existing Desktop folder, skips it
# with --no-desktop-shortcut, and --uninstall removes it.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT
BIN=rog-control-center-modern
APP_ID=io.github.anilsrml.RogControlModern

mkdir -p "$T/pkg/bin" "$T/pkg/share/applications" "$T/pkg/share/icons/hicolor/512x512/apps"
cp /usr/bin/true "$T/pkg/bin/$BIN"
printf '[Desktop Entry]\nType=Application\nName=ROG Control\nExec=%s\n' "$BIN" > "$T/pkg/share/applications/$APP_ID.desktop"
: > "$T/pkg/share/icons/hicolor/512x512/apps/$BIN.png"
cp "$ROOT/packaging/install.sh" "$T/pkg/"
mkdir -p "$T/home/Desktop"
run() { HOME="$T/home" XDG_DATA_HOME="$T/home/.local/share" XDG_CONFIG_HOME="$T/home/.config" bash "$T/pkg/install.sh" "$@" >"$T/out" 2>&1 || { cat "$T/out"; exit 1; }; }
SHORTCUT="$T/home/Desktop/$APP_ID.desktop"

fail=0
run
[[ -x "$SHORTCUT" ]] || { echo "FAIL: no executable desktop shortcut"; fail=1; }
grep -q "^Exec=$T/home/.local/bin/$BIN" "$SHORTCUT" 2>/dev/null || { echo "FAIL: shortcut does not run the installed binary"; fail=1; }
run --uninstall
[[ -e "$SHORTCUT" ]] && { echo "FAIL: uninstall left the shortcut"; fail=1; }
run --no-desktop-shortcut
[[ -e "$SHORTCUT" ]] && { echo "FAIL: --no-desktop-shortcut still created it"; fail=1; }
[[ $fail == 0 ]] && echo "PASS: desktop shortcut is created, optional and removed on uninstall"
exit $fail
