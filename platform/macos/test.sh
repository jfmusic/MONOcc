#!/bin/bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
TEST_DIR="$ROOT_DIR/target/native-verification"
mkdir -p "$TEST_DIR/work" "$ROOT_DIR/target/swift-cache"
python3 - "$ROOT_DIR" "$TEST_DIR" <<'PYTHON'
from pathlib import Path
import sys
root,test=map(Path,sys.argv[1:])
source=(root/'platform/macos/MonogramProfiles.swift').read_text().split('@main struct MonogramProfilesApp')[0]
(test/'Verification.swift').write_text(source+(root/'platform/macos/VerificationMain.swift').read_text())
PYTHON
swiftc -parse-as-library -target arm64-apple-macosx14.0 -module-cache-path "$ROOT_DIR/target/swift-cache" "$TEST_DIR/Verification.swift" -o "$TEST_DIR/Verification" -framework SwiftUI -framework AppKit -framework ApplicationServices -framework ServiceManagement
cd "$TEST_DIR"
MONOGRAM_PROFILE_HOME="$TEST_DIR/profile-store" "$TEST_DIR/Verification"
