#!/bin/bash

set -e
echo "🚀 Starting Fast Forward build process..."

if diskutil list 2>/dev/null | grep -q "Fast Forward$" || [ -d "/Volumes/Fast Forward/" ]; then
    echo "🔄 Unmounting any existing Fast Forward disk images..."
    diskutil list 2>/dev/null | grep "Fast Forward$" | awk '{print $NF}' | xargs -I{} diskutil unmountDisk force {} >/dev/null 2>&1 || true
    [ -d "/Volumes/Fast Forward/" ] && hdiutil detach "/Volumes/Fast Forward/" -force >/dev/null 2>&1 || true
fi

generate_icons=false
mkdir -p icons >/dev/null 2>&1
for size in 16 32 128 256 512; do
    if [ ! -f "icons/icon_${size}x${size}.png" ] || [ "assets/app_icon.png" -nt "icons/icon_${size}x${size}.png" ] || \
        [ ! -f "icons/icon_${size}x${size}@2x.png" ] || [ "assets/app_icon.png" -nt "icons/icon_${size}x${size}@2x.png" ]; then
        generate_icons=true
        break
    fi
done

if [ "$generate_icons" = true ]; then
    echo "📐 Generating icon sizes..."
    for size in 16 32 128 256 512; do
        sips -z $size $size assets/app_icon.png --out icons/icon_${size}x${size}.png >/dev/null 2>&1
        sips -z $((size*2)) $((size*2)) assets/app_icon.png --out icons/icon_${size}x${size}@2x.png >/dev/null 2>&1
    done
fi

echo "🔨 Building for multiple architectures..."
rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null 2>&1
cargo build --target=x86_64-apple-darwin --release >/dev/null 2>&1
cargo build --target=aarch64-apple-darwin --release >/dev/null 2>&1

echo "🔄 Creating universal binary..."
mkdir -p target/universal-apple-darwin/release/ >/dev/null 2>&1
lipo -create -output target/universal-apple-darwin/release/fast-forward \
    target/x86_64-apple-darwin/release/fast-forward \
    target/aarch64-apple-darwin/release/fast-forward >/dev/null 2>&1

echo "📦 Packaging the application..."
cargo install cargo-packager --locked >/dev/null 2>&1
cargo packager --release >/dev/null 2>&1

echo "✅ Build completed successfully!"
