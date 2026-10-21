#!/usr/bin/env bash
# install.sh must stop a running copy of the previously installed binary,
# and must not touch other processes. Uses a throwaway HOME and `sleep`
# standing in for the app binary.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
T="$(mktemp -d)"
trap 'kill $OTHER 2>/dev/null || true; rm -rf "$T"' EXIT
BIN=rog-control-center-modern
APP_ID=io.github.anilsrml.RogControlModern

# release layout with a fake binary
mkdir -p "$T/pkg/bin" "$T/pkg/share/applications" "$T/pkg/share/icons/hicolor/512x512/apps"
: > "$T/pkg/share/icons/hicolor/512x512/apps/$BIN.png"
cp "$(command -v sleep)" "$T/pkg/bin/$BIN"
printf '[Desktop Entry]\nType=Application\nName=Test\nExec=%s\n' "$BIN" > "$T/pkg/share/applications/$APP_ID.desktop"
cp "$ROOT/packaging/install.sh" "$T/pkg/"

# an "old install" that is running, plus an unrelated process with the same name elsewhere
mkdir -p "$T/home/.local/bin" "$T/other"
cp "$(command -v sleep)" "$T/home/.local/bin/$BIN"
cp "$(command -v sleep)" "$T/other/$BIN"
"$T/home/.local/bin/$BIN" 300 & OLD=$!
"$T/other/$BIN" 300 & OTHER=$!

HOME="$T/home" XDG_DATA_HOME="$T/home/.local/share" bash "$T/pkg/install.sh" >"$T/out" 2>&1 \
    || { echo "FAIL: install.sh exited non-zero"; cat "$T/out"; exit 1; }

fail=0
if kill -0 $OLD 2>/dev/null; then echo "FAIL: running old install was not stopped"; kill $OLD; fail=1; fi
if ! kill -0 $OTHER 2>/dev/null; then echo "FAIL: unrelated process was stopped"; fail=1; fi
grep -q "Stopped the running" "$T/out" || { echo "FAIL: no stop message"; cat "$T/out"; fail=1; }
[[ $fail == 0 ]] && echo "PASS: install stops only the installed running copy"
exit $fail
