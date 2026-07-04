#!/bin/bash
# UsageMeter를 더블클릭 실행 가능한 .app 번들로 패키징한다. (v1.0.0)
# 사용: ./build-app.sh  → UsageMeter.app 생성 (앱 아이콘 = icon.png)
set -e
cd "$(dirname "$0")"

VERSION="1.0.0"
BUILD="1"

echo "▶ 릴리즈 빌드…"
swift build -c release

APP="UsageMeter.app"
BIN=".build/release/UsageMeter"

echo "▶ 앱 아이콘(icon.png → AppIcon.icns) 생성…"
if [ -f icon.png ]; then
    ICONSET="AppIcon.iconset"
    rm -rf "$ICONSET"; mkdir -p "$ICONSET"
    sips -z 16 16     icon.png --out "$ICONSET/icon_16x16.png"      >/dev/null
    sips -z 32 32     icon.png --out "$ICONSET/icon_16x16@2x.png"   >/dev/null
    sips -z 32 32     icon.png --out "$ICONSET/icon_32x32.png"      >/dev/null
    sips -z 64 64     icon.png --out "$ICONSET/icon_32x32@2x.png"   >/dev/null
    sips -z 128 128   icon.png --out "$ICONSET/icon_128x128.png"    >/dev/null
    sips -z 256 256   icon.png --out "$ICONSET/icon_128x128@2x.png" >/dev/null
    sips -z 256 256   icon.png --out "$ICONSET/icon_256x256.png"    >/dev/null
    sips -z 512 512   icon.png --out "$ICONSET/icon_256x256@2x.png" >/dev/null
    sips -z 512 512   icon.png --out "$ICONSET/icon_512x512.png"    >/dev/null
    cp icon.png                    "$ICONSET/icon_512x512@2x.png"
    iconutil -c icns "$ICONSET" -o AppIcon.icns
    rm -rf "$ICONSET"
else
    echo "⚠ icon.png 없음 — 아이콘 없이 진행"
fi

echo "▶ $APP 번들 구성…"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/UsageMeter"
chmod +x "$APP/Contents/MacOS/UsageMeter"
[ -f AppIcon.icns ] && cp AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>            <string>UsageMeter</string>
    <key>CFBundleDisplayName</key>     <string>UsageMeter</string>
    <key>CFBundleIdentifier</key>      <string>com.usagemeter.app</string>
    <key>CFBundleExecutable</key>      <string>UsageMeter</string>
    <key>CFBundlePackageType</key>     <string>APPL</string>
    <key>CFBundleShortVersionString</key><string>${VERSION}</string>
    <key>CFBundleVersion</key>         <string>${BUILD}</string>
    <key>CFBundleIconFile</key>        <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>  <string>14.0</string>
    <key>LSUIElement</key>             <true/>
    <key>NSHighResolutionCapable</key> <true/>
</dict>
</plist>
PLIST

# 로컬 실행용 임시 서명(Gatekeeper 경고 완화, 없어도 동작).
codesign --force --deep --sign - "$APP" 2>/dev/null || true

echo "✅ 완료: $(pwd)/$APP  (v${VERSION})"
echo "   더블클릭하거나  open $APP  로 실행하세요."
