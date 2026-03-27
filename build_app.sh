#!/bin/bash
set -e

APP_NAME="SentinelRS"
APP_DIR="target/release/$APP_NAME.app"
BIN_DIR="$APP_DIR/Contents/MacOS"

echo "Building release binary..."
cargo build --release

echo "Creating App Bundle structure..."
mkdir -p "$BIN_DIR"

echo "Copying binary..."
cp target/release/sentinel-rs "$BIN_DIR/$APP_NAME"

echo "Creating Info.plist..."
cat << PLIST > "$APP_DIR/Contents/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>com.thusby.sentinelrs</string>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>LSUIElement</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
</dict>
</plist>
PLIST

echo "App Bundle created at: $APP_DIR"
