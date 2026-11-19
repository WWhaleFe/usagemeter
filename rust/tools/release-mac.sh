#!/bin/bash
# macOS 정식판: UI 빌드, 유니버설 번들, ad-hoc 서명, 배포 zip 생성. 앱은 실행하지 않는다.
set -euo pipefail
cd "$(dirname "$0")/.."
workspace="$(pwd)"
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"

cd crates/um-app/ui-src
npm ci
npm run build

cd "$workspace/crates/um-app"
npx -y @tauri-apps/cli@2.12.4 build --target universal-apple-darwin --bundles app --features production --config "$workspace/crates/um-app/tauri.release.conf.json"

app="$workspace/target/universal-apple-darwin/release/bundle/macos/UsageMeter.app"
codesign --force --deep --sign - "$app"
codesign --verify --deep --strict "$app"
binary="$app/Contents/MacOS/usagemeter"
architectures="$(lipo -info "$binary")"
printf '%s\n' "$architectures"
for arch in arm64 x86_64; do
  if [[ " $architectures " != *" $arch "* ]]; then
    printf '필수 아키텍처 없음: %s\n' "$arch" >&2
    exit 1
  fi
done

# Cargo 워크스페이스 버전을 파일명에 쓰고 번들의 표시 버전도 같은지 확인한다.
version="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\([^"]*\)"/\1/p' "$workspace/Cargo.toml")"
test -n "$version"
test "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app/Contents/Info.plist")" = "$version"
mkdir -p "$workspace/dist"
# Homebrew 탭 자동 갱신(bump.sh)과 1.5.0의 업데이트가 이 이름을 기대한다.
archive="$workspace/dist/UsageMeter-v$version.zip"
# 재실행할 때 이전 zip의 항목이 남지 않게 새 파일로 만든다.
rm -f "$archive"
ditto -c -k --keepParent "$app" "$archive"
printf '\n결과: %s\n크기: %s 바이트\n' "$archive" "$(stat -f%z "$archive")"
