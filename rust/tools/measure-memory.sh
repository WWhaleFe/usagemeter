#!/bin/bash
# 메모리 측정(macOS): 대상 프로세스 + 실행 뒤 새로 생긴 WebKit 프로세스의 footprint 합.
# 사용: measure-memory.sh <실행파일> <총 초> <간격 초> [로그]
# 계획서 4-4. WebKit 하위 프로세스는 launchd 자식이라 실행 전 목록과 비교해 찾는다.
set -u
BIN="$1"; TOTAL="$2"; STEP="$3"; LOG="${4:-/dev/null}"
wk() { pgrep -f 'com.apple.WebKit\.(WebContent|Networking|GPU)' | sort; }
fp() { footprint -p "$1" 2>/dev/null | awk '/Footprint:/{for(i=1;i<=NF;i++) if($i=="Footprint:"){v=$(i+1); u=$(i+2); if(u=="KB") v=v/1024; if(u=="GB") v=v*1024; print v; exit}}'; }
before=$(wk)
"$BIN" >"$LOG" 2>&1 &
PID=$!
for ((t=STEP; t<=TOTAL; t+=STEP)); do
  sleep "$STEP"
  main=$(fp $PID); sum=${main:-0}; extra=""
  for p in $(comm -13 <(echo "$before") <(wk)); do
    m=$(fp $p); [ -n "$m" ] && { sum=$(echo "$sum + $m" | bc); extra="$extra $(ps -o comm= -p $p | sed 's/.*WebKit\.//'):${m}"; }
  done
  printf "t=%4ss  본체 %6s MB  합계 %7s MB %s\n" "$t" "${main:-?}" "$sum" "$extra"
done
kill $PID 2>/dev/null
