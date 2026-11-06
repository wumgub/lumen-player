#!/bin/bash
# Build Lumen.app for macOS from the release binary, with libmpv and everything
# it links copied into Contents/Frameworks (via dylibbundler) so the app runs
# on a Mac without Homebrew. Usage: scripts/bundle-macos.sh [out-dir]
set -euo pipefail
cd "$(dirname "$0")/.."
OUT="${1:-target/bundle}"
VERSION=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
APP="$OUT/Lumen.app"

cargo build --release
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP/Contents/Frameworks"
cp target/release/lumen "$APP/Contents/MacOS/lumen"
cp build/icon.icns "$APP/Contents/Resources/icon.icns"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Lumen</string>
  <key>CFBundleDisplayName</key><string>Lumen</string>
  <key>CFBundleIdentifier</key><string>com.lumen.player</string>
  <key>CFBundleExecutable</key><string>lumen</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.music</string>
</dict>
</plist>
PLIST

# Copy libmpv and its dependency tree in, rewriting load paths to the bundle.
dylibbundler -of -cd -b -x "$APP/Contents/MacOS/lumen" \
  -d "$APP/Contents/Frameworks" -p @executable_path/../Frameworks/ >/dev/null

# dylibbundler can add the same LC_RPATH twice, which current dyld refuses to
# load. Drop the duplicates.
for f in "$APP/Contents/MacOS/lumen" "$APP/Contents/Frameworks/"*.dylib; do
  otool -l "$f" | awk '/LC_RPATH/{getline; getline; print $2}' | sort | uniq -d |
    while read -r rp; do install_name_tool -delete_rpath "$rp" "$f" 2>/dev/null; done
done

# install_name_tool invalidates signatures; Apple Silicon refuses to load
# unsigned code, so sign everything ad hoc.
codesign --force --sign - "$APP/Contents/Frameworks/"*.dylib >/dev/null 2>&1
codesign --force --sign - "$APP" >/dev/null 2>&1
echo "$APP"
