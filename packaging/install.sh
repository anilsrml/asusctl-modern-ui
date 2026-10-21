#!/usr/bin/env bash
# User-local installer for ROG Control (Modern). No sudo needed.
# Usage: ./install.sh [--uninstall]
# Run from the extracted release tarball (or from packaging/ of a built tree
# with the same layout: bin/, share/).
set -euo pipefail

BIN=rog-control-center-modern
APP_ID=io.github.anilsrml.RogControlModern
DOMAIN=rog-control-center-modern

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN_DIR="$HOME/.local/bin"
APPS_DIR="$DATA_HOME/applications"
ICON_DIR="$DATA_HOME/icons/hicolor/512x512/apps"
LOCALE_DIR="$DATA_HOME/locale"

# PIDs of this user's processes running the installed binary, including a
# copy whose file was already replaced ("(deleted)")
installed_pids() {
    local pid exe
    for pid in $(pgrep -u "$(id -u)" -x "${BIN:0:15}" 2>/dev/null); do
        exe="$(readlink "/proc/$pid/exe" 2>/dev/null)" || continue
        [[ "$exe" == "$BIN_DIR/$BIN" || "$exe" == "$BIN_DIR/$BIN (deleted)" ]] && echo "$pid"
    done
}

# Stop running copies so the next launch uses the new binary instead of
# handing off to the old single instance. Prints whether one was running.
stop_running() {
    local pids
    pids="$(installed_pids)"
    [[ -z "$pids" ]] && return 1
    kill $pids 2>/dev/null || true
    for _ in 1 2 3 4 5 6 7 8 9 10; do
        [[ -z "$(installed_pids)" ]] && break
        sleep 0.3
    done
    [[ -n "$(installed_pids)" ]] && kill -9 $(installed_pids) 2>/dev/null || true
    echo "Stopped the running ROG Control (Modern)."
    return 0
}

uninstall() {
    stop_running || true
    rm -f "$BIN_DIR/$BIN" \
        "$APPS_DIR/$APP_ID.desktop" \
        "$ICON_DIR/$BIN.png"
    rm -f "$LOCALE_DIR"/*/LC_MESSAGES/"$DOMAIN".mo
    rm -f "$HOME/.config/autostart/$APP_ID.desktop"
    command -v update-desktop-database >/dev/null && update-desktop-database "$APPS_DIR" 2>/dev/null || true
    command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$DATA_HOME/icons/hicolor" 2>/dev/null || true
    echo "Uninstalled ROG Control (Modern). Settings in ~/.config/rog-modern/ were kept."
}

if [[ "${1:-}" == "--uninstall" ]]; then
    uninstall
    exit 0
elif [[ $# -gt 0 ]]; then
    echo "usage: $0 [--uninstall]" >&2
    exit 2
fi

[[ -x "$HERE/bin/$BIN" ]] || { echo "error: $HERE/bin/$BIN not found; run from the release tarball" >&2; exit 1; }

was_running=0
stop_running && was_running=1

install -Dm755 "$HERE/bin/$BIN" "$BIN_DIR/$BIN"
install -Dm644 "$HERE/share/applications/$APP_ID.desktop" "$APPS_DIR/$APP_ID.desktop"
# make the launcher independent of $PATH
sed -i "s|^Exec=.*|Exec=$BIN_DIR/$BIN|" "$APPS_DIR/$APP_ID.desktop"
for icon in "$HERE"/share/icons/hicolor/512x512/apps/*.png; do
    install -Dm644 "$icon" "$ICON_DIR/$(basename "$icon")"
done
for mo in "$HERE"/share/locale/*/LC_MESSAGES/"$DOMAIN".mo; do
    [[ -f "$mo" ]] || continue
    lang="$(basename "$(dirname "$(dirname "$mo")")")"
    install -Dm644 "$mo" "$LOCALE_DIR/$lang/LC_MESSAGES/$DOMAIN.mo"
done
command -v update-desktop-database >/dev/null && update-desktop-database "$APPS_DIR" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$DATA_HOME/icons/hicolor" 2>/dev/null || true

echo "Installed to $BIN_DIR/$BIN"
if [[ $was_running == 1 ]]; then
    # Bring it back in the tray, as it was before the update
    nohup "$BIN_DIR/$BIN" --background >/dev/null 2>&1 &
    echo "Restarted it in the tray with the new version."
fi
case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "note: $BIN_DIR is not in your PATH; use the app menu entry or add it." ;;
esac

if ! systemctl is-active --quiet asusd 2>/dev/null; then
    echo "WARNING: asusd is not running. This app only talks to the asusd daemon." >&2
    echo "         Install asusctl/asusd and run: sudo systemctl enable --now asusd" >&2
fi
