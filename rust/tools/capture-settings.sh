#!/bin/bash
# 설정 창 탭 전부와 분석 창을 캡처한다(맥 화면 비교용). 사용: tools/capture-settings.sh <출력 폴더> [탭...]
# 미리 cargo build -p um-app --release. 창 번호는 tools/winid(CGWindowList)로 크기를 보고 고른다.
set -u
cd "$(dirname "$0")/.."
OUT=${1:?출력 폴더}; shift
mkdir -p "$OUT"
TABS=${*:-"login presets display interval line subItems partition layout corner notch support"}
[ -x tools/winid ] || swiftc -O tools/winid.swift -o tools/winid
for tab in $TABS; do
  UM_HEADLESS=1 UM_OPEN_WINDOW="settings#$tab" ./target/release/usagemeter >/dev/null 2>&1 & P=$!
  sleep 5
  ID=$(tools/winid $P | awk '$2==760{print $1}' | head -1)
  [ -n "$ID" ] && screencapture -x -o -l "$ID" "$OUT/rs_$tab.png" || echo "캡처 실패: $tab"
  kill $P; wait $P 2>/dev/null
done
UM_HEADLESS=1 UM_OPEN_WINDOW=analytics ./target/release/usagemeter >/dev/null 2>&1 & P=$!
sleep 7
ID=$(tools/winid $P | awk '$2==1000{print $1}' | head -1)
[ -n "$ID" ] && screencapture -x -o -l "$ID" "$OUT/rs_analytics.png"
kill $P; wait $P 2>/dev/null
ls "$OUT"
