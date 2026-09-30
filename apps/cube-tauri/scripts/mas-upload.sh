#!/usr/bin/env bash
# Build a Mac App Store .pkg and upload it with altool.
# Run from repo root or from apps/cube-tauri.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"          # apps/cube-tauri
APP_NAME="FaceTurn Cube Solver"
BUNDLE_ID="com.dougcoburn.faceturn"
TEAM_ID="3SNVPB52FQ"
APP_IDENTITY="${APPLE_SIGNING_IDENTITY:-3rd Party Mac Developer Application: DOUGLAS KELLY COBURN (${TEAM_ID})}"
INSTALLER_IDENTITY="${APPLE_INSTALLER_IDENTITY:-3rd Party Mac Developer Installer: DOUGLAS KELLY COBURN (${TEAM_ID})}"
PROFILE="${MAS_PROVISION_PROFILE:-$ROOT/FaceTurn_Cube_Solver_ProvProf.provisionprofile}"
APP="$ROOT/src-tauri/target/release/bundle/macos/${APP_NAME}.app"
STAMP="$(date +%Y%m%d-%H%M%S)"
PKG="${MAS_PKG:-$HOME/Desktop/FaceTurn-${STAMP}.pkg}"

need() { command -v "$1" >/dev/null || { echo "missing $1"; exit 1; }; }
need npx; need codesign; need productbuild; need xcrun

: "${APPLE_API_KEY_ID:?set APPLE_API_KEY_ID}"
: "${APPLE_API_ISSUER:?set APPLE_API_ISSUER}"
[[ -f "$HOME/.appstoreconnect/private_keys/AuthKey_${APPLE_API_KEY_ID}.p8" ]] \
  || { echo "missing ~/.appstoreconnect/private_keys/AuthKey_${APPLE_API_KEY_ID}.p8"; exit 1; }
[[ -f "$PROFILE" ]] || { echo "missing profile: $PROFILE"; exit 1; }

if ! security find-identity -p codesigning -v | grep -F -q "$APP_IDENTITY"; then
  echo "codesigning identity not in login keychain:"
  echo "  $APP_IDENTITY"
  security find-identity -p codesigning -v
  exit 1
fi

cd "$ROOT"
export APPLE_SIGNING_IDENTITY="$APP_IDENTITY"
npx tauri build --bundles app

[[ -d "$APP" ]] || { echo "app not built: $APP"; exit 1; }
[[ -f "$APP/Contents/embedded.provisionprofile" ]] || {
  echo "embedded.provisionprofile missing — check tauri.conf.json files path"
  exit 1
}

# Re-stamp entitlements on binary + wrapper (Tauri sometimes signs only one).
codesign --force --options runtime --timestamp \
  --entitlements "$ROOT/src-tauri/Entitlements.plist" \
  --sign "$APP_IDENTITY" \
  "$APP/Contents/MacOS/cube-tauri"
codesign --force --options runtime --timestamp \
  --entitlements "$ROOT/src-tauri/Entitlements.plist" \
  --sign "$APP_IDENTITY" \
  "$APP"

ent() { codesign -d --entitlements - "$1" 2>/dev/null | grep -q app-sandbox; }
ent "$APP" || { echo "sandbox missing on .app"; exit 1; }
ent "$APP/Contents/MacOS/cube-tauri" || { echo "sandbox missing on cube-tauri"; exit 1; }

xcrun productbuild \
  --sign "$INSTALLER_IDENTITY" \
  --component "$APP" /Applications \
  "$PKG"

echo "uploading $PKG"
xcrun altool --upload-app --type macos \
  --file "$PKG" \
  --apiKey "$APPLE_API_KEY_ID" \
  --apiIssuer "$APPLE_API_ISSUER"

echo "ok: $PKG"
echo "Connect → FaceTurn → next version → select the new build when processing finishes."
echo "Bump bundle version in src-tauri/tauri.conf.json before the next run."
