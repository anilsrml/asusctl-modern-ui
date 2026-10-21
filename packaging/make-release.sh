#!/usr/bin/env bash
# Build the release binary and assemble dist/asusctl-modern-ui-<version>-x86_64-linux.tar.gz
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
ROOT="$PWD"
BIN=rog-control-center-modern
APP_ID=io.github.anilsrml.RogControlModern
DOMAIN=rog-control-center-modern

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
NAME="asusctl-modern-ui-$VERSION-x86_64-linux"
STAGE="$ROOT/dist/$NAME"

cargo build -j 4 --release -p rog-control-center

rm -rf "$STAGE"
mkdir -p "$STAGE/bin" "$STAGE/share/applications" "$STAGE/share/icons/hicolor/512x512/apps"
install -m755 "target/release/$BIN" "$STAGE/bin/$BIN"
strip -s "$STAGE/bin/$BIN" || true

install -m644 "rog-control-center/data/$APP_ID.desktop" "$STAGE/share/applications/"
install -m644 "rog-control-center/data/$BIN.png" "$STAGE/share/icons/hicolor/512x512/apps/"
# tray status icons (normally shipped by upstream asusctl; bundled for user-local installs)
install -m644 data/icons/asus_notif_*.png "$STAGE/share/icons/hicolor/512x512/apps/"

# newest compiled catalogs produced by build.rs
OUT="$(ls -dt target/release/build/rog-control-center-*/out/translations 2>/dev/null | head -1)"
[[ -n "$OUT" ]] || { echo "error: compiled translations not found" >&2; exit 1; }
for mo in "$OUT"/*/LC_MESSAGES/$DOMAIN.mo; do
    lang="$(basename "$(dirname "$(dirname "$mo")")")"
    install -Dm644 "$mo" "$STAGE/share/locale/$lang/LC_MESSAGES/$DOMAIN.mo"
done

install -m755 packaging/install.sh "$STAGE/install.sh"
install -m644 LICENSE NOTICE README.md "$STAGE/"

tar -C dist -czf "dist/$NAME.tar.gz" "$NAME"
rm -rf "$STAGE"
echo "Created dist/$NAME.tar.gz"
( cd dist && sha256sum "$NAME.tar.gz" )
