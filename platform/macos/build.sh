#!/bin/bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
DEST_DIR="${1:-$ROOT_DIR/dist}"
APP_DIR="$DEST_DIR/MONOcc.app"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources" "$ROOT_DIR/target/swift-cache"
swiftc -parse-as-library -target arm64-apple-macosx14.0 -module-cache-path "$ROOT_DIR/target/swift-cache" "$ROOT_DIR/platform/macos/MonogramProfiles.swift" -o "$APP_DIR/Contents/MacOS/MonogramProfiles" -framework SwiftUI -framework AppKit -framework ApplicationServices -framework ServiceManagement
cp "$ROOT_DIR/target/release/sextant" "$APP_DIR/Contents/Resources/sextant"
cp "$ROOT_DIR/platform/macos/MONOcc.icns" "$APP_DIR/Contents/Resources/MONOcc.icns"
cp "$ROOT_DIR/LICENSE" "$APP_DIR/Contents/Resources/LICENSE-sextant.txt"
cat > "$APP_DIR/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>MONOcc</string>
<key>CFBundleDisplayName</key><string>MONOcc</string>
<key>CFBundleIdentifier</key><string>org.monocc.studio</string>
<key>CFBundleExecutable</key><string>MonogramProfiles</string>
<key>CFBundleIconFile</key><string>MONOcc</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.15</string>
<key>CFBundleVersion</key><string>16</string>
<key>LSMinimumSystemVersion</key><string>14.0</string>
<key>LSUIElement</key><true/>
<key>NSHighResolutionCapable</key><true/>
<key>NSAppleEventsUsageDescription</key><string>Apply the output volume action you assign to a slider.</string>
</dict></plist>
PLIST
HELPER_DIR="$APP_DIR/Contents/Library/LoginItems/MONOcc Login.app"
mkdir -p "$HELPER_DIR/Contents/MacOS"
swiftc -parse-as-library -target arm64-apple-macosx14.0 -module-cache-path "$ROOT_DIR/target/swift-cache" "$ROOT_DIR/platform/macos/LoginHelper.swift" -o "$HELPER_DIR/Contents/MacOS/MONOccLogin" -framework AppKit
cat > "$HELPER_DIR/Contents/Info.plist" <<'HELPERPLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>MONOcc Login</string>
<key>CFBundleIdentifier</key><string>org.monocc.login</string>
<key>CFBundleExecutable</key><string>MONOccLogin</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.15</string>
<key>CFBundleVersion</key><string>16</string>
<key>LSUIElement</key><true/>
<key>LSMinimumSystemVersion</key><string>14.0</string>
</dict></plist>
HELPERPLIST
codesign --force --sign - "$HELPER_DIR"
codesign --force --sign - "$APP_DIR/Contents/Resources/sextant"
xattr -d com.apple.FinderInfo "$APP_DIR" 2>/dev/null || true
codesign --force --deep --sign - "$APP_DIR"
printf '%s\n' "$APP_DIR"
