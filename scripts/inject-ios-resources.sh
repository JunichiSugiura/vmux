#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/scripts/cargo-target-paths.sh"

PROFILE="${VMUX_IOS_PROFILE:-debug}"
case "$PROFILE" in
    debug | release) ;;
    *)
        echo "Error: unknown VMUX_IOS_PROFILE=$PROFILE (expected debug|release)" >&2
        exit 1
        ;;
esac

if ! xcrun --find actool >/dev/null 2>&1 || ! xcrun --find ibtool >/dev/null 2>&1; then
    echo "Error: actool/ibtool not found. Install Xcode (not just the Command Line Tools)." >&2
    exit 1
fi

TARGET_DIR="$(vmux_cargo_target_dir "$ROOT")"
DEFAULT_BUNDLE="$TARGET_DIR/dx/vmux_mobile/$PROFILE/ios/VmuxMobile.app"
APP_BUNDLE="${1:-${VMUX_IOS_APP_BUNDLE:-$DEFAULT_BUNDLE}}"

if [[ ! -d "$APP_BUNDLE" ]]; then
    echo "Error: no .app at $APP_BUNDLE. Run 'make mobile-ios' first." >&2
    exit 1
fi

PLIST="$APP_BUNDLE/Info.plist"
if [[ ! -f "$PLIST" ]]; then
    echo "Error: no Info.plist in $APP_BUNDLE." >&2
    exit 1
fi

PACKAGING="$ROOT/packaging/ios"
DEPLOYMENT_TARGET="15.0"

STAGE="$(mktemp -d -t vmux-ios-assets)"
cleanup() { rm -rf "$STAGE"; }
trap cleanup EXIT

xcrun actool "$PACKAGING/Assets.xcassets" \
    --compile "$STAGE" \
    --platform iphoneos \
    --target-device iphone \
    --minimum-deployment-target "$DEPLOYMENT_TARGET" \
    --app-icon AppIcon \
    --output-partial-info-plist "$STAGE/partial.plist" \
    --output-format human-readable-text >/dev/null

xcrun ibtool "$PACKAGING/LaunchScreen.storyboard" \
    --compile "$APP_BUNDLE/LaunchScreen.storyboardc" \
    --target-device iphone \
    --minimum-deployment-target "$DEPLOYMENT_TARGET" \
    --output-format human-readable-text >/dev/null

cp -f "$STAGE/Assets.car" "$APP_BUNDLE/Assets.car"
for icon in "$STAGE"/AppIcon*.png; do
    case "$icon" in
        *'~ipad.png') continue ;;
    esac
    cp -f "$icon" "$APP_BUNDLE/"
done

cp -f "$PACKAGING/PrivacyInfo.xcprivacy" "$APP_BUNDLE/PrivacyInfo.xcprivacy"

VERSION="$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
BUILD_NUMBER="${VMUX_IOS_BUILD_NUMBER:-1}"

if [[ -z "$VERSION" ]]; then
    echo "Error: could not read version from $ROOT/Cargo.toml" >&2
    exit 1
fi

/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" "$PLIST"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $BUILD_NUMBER" "$PLIST"

set_plist() {
    /usr/libexec/PlistBuddy -c "Delete :$1" "$PLIST" >/dev/null 2>&1 || true
    /usr/libexec/PlistBuddy -c "Add :$1 string $2" "$PLIST"
}

SDK_VERSION="$(xcrun --sdk iphoneos --show-sdk-version)"
SDK_BUILD="$(xcrun --sdk iphoneos --show-sdk-build-version)"
XCODE_INFO="$(xcodebuild -version)"
XCODE_VERSION="$(awk 'NR == 1 { print $2 }' <<<"$XCODE_INFO")"
XCODE_BUILD="$(awk 'END { print $3 }' <<<"$XCODE_INFO")"
XCODE_PADDED="$(printf '%d%d0' "${XCODE_VERSION%%.*}" "$(echo "$XCODE_VERSION" | cut -d. -f2)")"

set_plist CFBundleIconName AppIcon
set_plist MinimumOSVersion "$DEPLOYMENT_TARGET"
set_plist DTPlatformName iphoneos
set_plist DTPlatformVersion "$SDK_VERSION"
set_plist DTPlatformBuild "$SDK_BUILD"
set_plist DTSDKName "iphoneos$SDK_VERSION"
set_plist DTSDKBuild "$SDK_BUILD"
set_plist DTXcode "$XCODE_PADDED"
set_plist DTXcodeBuild "$XCODE_BUILD"
set_plist BuildMachineOSBuild "$(sw_vers -buildVersion)"

echo "inject-ios-resources: $APP_BUNDLE (version $VERSION, build $BUILD_NUMBER)"
