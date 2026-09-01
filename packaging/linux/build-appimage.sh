#!/bin/sh
# build-appimage.sh — build codewig-live AppImage on a Linux machine.
#
# Usage:
#   packaging/linux/build-appimage.sh [BIN_DIR]
#
# BIN_DIR must contain codewig-live and codewig-cli (e.g. copied over from a
# Windows cross-build: target/x86_64-unknown-linux-gnu/release).
# Defaults: ../../target/x86_64-unknown-linux-gnu/release, then ../../target/release.
#
# Needs: curl, plus FUSE on the machine (or appimagetool runs via
# --appimage-extract-and-run fallback automatically).
set -eu

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN_DIR="${1:-}"
if [ -z "$BIN_DIR" ]; then
    if [ -x "$REPO_ROOT/target/x86_64-unknown-linux-gnu/release/codewig-live" ]; then
        BIN_DIR="$REPO_ROOT/target/x86_64-unknown-linux-gnu/release"
    else
        BIN_DIR="$REPO_ROOT/target/release"
    fi
fi

[ -x "$BIN_DIR/codewig-live" ] || { echo "error: $BIN_DIR/codewig-live not found/executable" >&2; exit 1; }

# Work in a native Linux fs: drvfs (/mnt/*) cannot store exec bits, and
# appimagetool aborts when the final chmod fails. Result is copied back at the end.
OUT_NAME="codewig-live-x86_64.AppImage"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
OUT="$WORK/$OUT_NAME"
APPDIR="$WORK/AppDir"
TOOL="${APPIMAGETOOL:-$HOME/.local/bin/appimagetool}"

rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps" \
         "$APPDIR/usr/share/icons/hicolor/512x512/apps" \
         "$APPDIR/usr/share/codewig/devices"

cp "$BIN_DIR/codewig-live" "$APPDIR/usr/bin/"
[ ! -x "$BIN_DIR/codewig-cli" ] || cp "$BIN_DIR/codewig-cli" "$APPDIR/usr/bin/"
cp "$REPO_ROOT/packaging/linux/codewig-live.desktop" "$APPDIR/usr/share/applications/"
cp "$REPO_ROOT/assets/icon-256.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/codewig-live.png"
cp "$REPO_ROOT/assets/icon-512.png" "$APPDIR/usr/share/icons/hicolor/512x512/apps/codewig-live.png"
cp "$REPO_ROOT/devices/aliases.yml" "$REPO_ROOT/devices/README.md" "$APPDIR/usr/share/codewig/devices/"
cp "$REPO_ROOT/packaging/linux/codewig-live.desktop" "$APPDIR/"
cp "$REPO_ROOT/assets/icon-256.png" "$APPDIR/codewig-live.png"
# AppRun = entry point the AppImage runtime execs (missing AppRun = "execv error")
printf '#!/bin/sh\nHERE="$(dirname "$(readlink -f "$0")")"\nexec "$HERE/usr/bin/codewig-live" "$@"\n' > "$APPDIR/AppRun"
chmod +x "$APPDIR/AppRun"

if [ ! -x "$TOOL" ]; then
    echo "fetching appimagetool -> $TOOL"
    mkdir -p "$(dirname "$TOOL")"
    curl -sL -o "$TOOL" \
        https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x "$TOOL"
fi

if ! ARCH=x86_64 "$TOOL" "$APPDIR" "$OUT" 2>/dev/null; then
    # no FUSE — run the tool itself via extract-and-run
    ARCH=x86_64 "$TOOL" --appimage-extract-and-run "$APPDIR" "$OUT"
fi

chmod +x "$OUT"
cp "$OUT" "$REPO_ROOT/$OUT_NAME"
echo "built: $REPO_ROOT/$OUT_NAME"
