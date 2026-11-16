#!/bin/bash
# 24시간 측정(rust/target/s3-24h) 요약: 직접 호출 성공률(AI별), 웹뷰 대체 횟수, 메모리 처음·최대·마지막.
D="$(dirname "$0")/../target/s3-24h"
[ -f "$D/app.log" ] || { echo "측정 로그 없음: $D"; exit 1; }
cat "$D/README.txt"
L=$(sed 's/\x1b\[[0-9;]*m//g' "$D/app.log")
for id in claude codex gemini; do
  ok=$(grep -c "$id 직접 조회 .*ok=true" <<<"$L"); bad=$(grep -c "$id 직접 조회 실패" <<<"$L"); web=$(grep -c "$id 웹 조회" <<<"$L")
  tot=$((ok + bad)); pct=$([ $tot -gt 0 ] && echo "scale=1; $ok*100/$tot" | bc || echo "-")
  echo "$id: 직접 성공 ${ok} / ${tot} (${pct}%), 웹뷰 조회 ${web}회"
done
echo "agy(Gemini) 실패 후 웹 조회: $(grep -c 'gemini 웹 조회' <<<"$L")회"
awk '/합계/{match($0, /본체 +[0-9.]+/); b=substr($0, RSTART, RLENGTH); sub(/본체 +/, "", b); b+=0; match($0, /합계 +[0-9.]+/); t=substr($0, RSTART, RLENGTH); sub(/합계 +/, "", t); t+=0; if(fb=="")fb=b; if(b>mb)mb=b; lb=b; if(ft=="")ft=t; if(t>mt)mt=t; lt=t; n++} END{print "메모리 표본 " n "개(10분 간격)"; print "  앱 본체: 처음 " fb " MB, 최대 " mb " MB, 마지막 " lb " MB"; print "  합계(본체 + 측정 시작 뒤 생긴 WebKit 프로세스): 처음 " ft ", 최대 " mt ", 마지막 " lt " MB"}' "$D/memory.log"
echo "  주의: 측정 중 다른 앱(예: Swift 버전)을 다시 켜면 그 앱의 WebKit 프로세스도 합계에 섞인다. 누수 판단은 본체 값으로 한다."
