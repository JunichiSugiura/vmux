#!/usr/bin/env bash
set -euo pipefail

# Create a DMG from Vmux.app.
#
# Optional:
#   APP_BUNDLE  - Path to .app (default: build/Vmux.app)
#   VERSION     - Version string (default: read from Info.plist)

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_BUNDLE="${APP_BUNDLE:-$ROOT/build/Vmux.app}"
VERSION="${VERSION:-$(/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" "$APP_BUNDLE/Contents/Info.plist")}"
DMG_NAME="Vmux-${VERSION}-mac.dmg"
DMG_PATH="$ROOT/build/$DMG_NAME"

if [ ! -d "$APP_BUNDLE" ]; then
    echo "Error: $APP_BUNDLE not found." >&2
    exit 1
fi

VMUX_ICNS=""
if [ -f "$ROOT/packaging/macos/Vmux.icns" ]; then
    VMUX_ICNS="$ROOT/packaging/macos/Vmux.icns"
fi

echo "==> Creating DMG: $DMG_NAME"

# Use create-dmg if available (prettier result), otherwise fall back to hdiutil
if command -v create-dmg >/dev/null 2>&1; then
    # Remove existing DMG (create-dmg fails if it exists)
    rm -f "$DMG_PATH"

    create-dmg \
        --volname "Vmux" \
        ${VMUX_ICNS:+--volicon "$VMUX_ICNS"} \
        --window-pos 200 120 \
        --window-size 600 400 \
        --icon-size 100 \
        --icon "Vmux.app" 150 190 \
        --app-drop-link 450 190 \
        --hide-extension "Vmux.app" \
        "$DMG_PATH" \
        "$APP_BUNDLE"
else
    echo "  (create-dmg not found, using hdiutil fallback)"
    rm -f "$DMG_PATH"
    hdiutil create -volname "Vmux" -srcfolder "$APP_BUNDLE" \
        -ov -format UDZO "$DMG_PATH"
fi

echo "Done: $DMG_PATH"
