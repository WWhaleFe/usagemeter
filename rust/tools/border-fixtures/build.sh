#!/bin/bash
# 테두리 기준 이미지 도구를 빌드하고(옵션) 실행한다. macOS 전용, Xcode(swiftc) 필요.
#   ./build.sh             빌드만 (rust/target/border-fixtures/border-fixtures 생성)
#   ./build.sh run         빌드 후 cases.json 전체를 ../../crates/um-render/tests/fixtures/ref/ 에 렌더
#   ./build.sh run name..  지정한 케이스만 렌더
# BorderView.swift·OverlaySettings.swift·Provider.swift·Providers.swift·Localization.swift는 실제 파일을 컴파일하고,
# ProviderManager만 ProviderManagerStub.swift(WebView 연결부 제거)로 대체한다.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
SRC="$ROOT/Sources/UsageMeter"
OUT="$ROOT/rust/target/border-fixtures"
mkdir -p "$OUT"
swiftc -O -swift-version 5 -o "$OUT/border-fixtures" \
  "$SRC/BorderView.swift" "$SRC/OverlaySettings.swift" "$SRC/Provider.swift" \
  "$SRC/Providers.swift" "$SRC/Localization.swift" \
  "$HERE/ProviderManagerStub.swift" "$HERE/main.swift"
if [ "${1:-}" = "run" ]; then
  shift
  "$OUT/border-fixtures" "$HERE/cases.json" "$ROOT/rust/crates/um-render/tests/fixtures/ref" "$@"
fi
