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
awk '/합계/{gsub(/MB/,""); v=$5; if(first=="")first=v; if(v>max)max=v; last=v; n++} END{print "메모리 표본 " n "개: 처음 " first " MB, 최대 " max " MB, 마지막 " last " MB"}' "$D/memory.log"
