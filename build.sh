#!/bin/bash

# Exit on any error
set -e

echo "🚀 Starting Fast Forward build process..."

echo "📦 Checking required tools..."
if ! command -v create-dmg &> /dev/null; then
    echo "Installing create-dmg..."
    brew install create-dmg
fi

echo "🎨 Creating icon assets..."
mkdir -p icons.iconset

echo "📐 Generating icon sizes..."
for size in 16 32 128 256 512; do
    sips -z $size $size assets/app_icon.png --out icons.iconset/icon_${size}x${size}.png
    sips -z $((size*2)) $((size*2)) assets/app_icon.png --out icons.iconset/icon_${size}x${size}@2x.png
done

echo "🎯 Creating icns file..."
mkdir -p assets
iconutil -c icns icons.iconset -o assets/icon.icns
rm -rf icons.iconset

echo "🔨 Building release version..."
cargo build --release

echo "📦 Creating app bundle..."
# Create the bundle directory structure
BUNDLE_DIR="target/release/bundle/osx/Fast Forward.app"
mkdir -p "${BUNDLE_DIR}/Contents/MacOS"
mkdir -p "${BUNDLE_DIR}/Contents/MacOS/assets/icons"
mkdir -p "${BUNDLE_DIR}/Contents/Resources"

# Copy the binary
cp target/release/fast-forward "${BUNDLE_DIR}/Contents/MacOS/Fast Forward"

# Copy the icons
cp assets/icon.icns "${BUNDLE_DIR}/Contents/Resources/"
cp assets/icons/* "${BUNDLE_DIR}/Contents/MacOS/assets/icons/"

# Create Info.plist
cat > "${BUNDLE_DIR}/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleExecutable</key>
    <string>Fast Forward</string>
    <key>CFBundleIconFile</key>
    <string>icon</string>
    <key>CFBundleIdentifier</key>
    <string>com.gaauwe.fast-forward</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Fast Forward</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
</dict>
</plist>
EOF

echo "🔐 Signing the application..."
# First remove any existing extended attributes
xattr -cr "${BUNDLE_DIR}"

# Sign the binary
codesign --force --deep --sign - "${BUNDLE_DIR}/Contents/MacOS/Fast Forward"

# Sign the app bundle
codesign --force --deep --sign - "${BUNDLE_DIR}"

echo "💿 Creating DMG file..."
rm -f FastForward.dmg

if [ ! -d "${BUNDLE_DIR}" ]; then
    echo "Error: App bundle directory not found at ${BUNDLE_DIR}"
    exit 1
fi

create-dmg \
    --volname "Fast Forward" \
    --volicon "assets/icon.icns" \
    --window-pos 200 120 \
    --window-size 600 400 \
    --icon-size 100 \
    --icon "Fast Forward.app" 175 120 \
    --hide-extension "Fast Forward.app" \
    --app-drop-link 425 120 \
    "FastForward.dmg" \
    "${BUNDLE_DIR}/"

echo "🔐 Signing the DMG..."
codesign --force --sign - "FastForward.dmg"

echo "✨ Build complete! FastForward.dmg has been created and signed."

# Print some verification information
echo "🔍 Verification information:"
codesign -vv --deep --strict "${BUNDLE_DIR}"
codesign -vv "FastForward.dmg"

echo "🗑️ Removing the bundle directory..."
rm -rf "${BUNDLE_DIR}"
