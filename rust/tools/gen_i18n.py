#!/usr/bin/env python3
"""Swift Localization.swift 의 `Loc.table` 을 strings.json 으로 변환한다.

사용: python3 rust/tools/gen_i18n.py
토크나이저로 문자열 리터럴(이스케이프, 여러 줄 항목, 주석)을 읽고,
해석하지 못하는 항목이 있으면 즉시 실패한다.
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "Sources/UsageMeter/Localization.swift"
OUT = ROOT / "rust/crates/um-core/src/i18n/strings.json"

SIMPLE_ESC = {"n": "\n", "t": "\t", "r": "\r", "0": "\0", "\\": "\\", '"': '"', "'": "'"}


class ParseError(Exception):
    pass


def fail(src, pos, msg):
    line = src.count("\n", 0, pos) + 1
    raise ParseError(f"{SRC.name}:{line}: {msg}")


def tokenize(src, start):
    """start(여는 `[` 다음)부터 닫는 `]` 까지 토큰을 낸다. 토큰: (종류, 값, 위치)."""
    i, n = start, len(src)
    while i < n:
        c = src[i]
        if c.isspace():
            i += 1
        elif src.startswith("//", i):
            j = src.find("\n", i)
            i = n if j < 0 else j
        elif src.startswith("/*", i):
            depth, i = 1, i + 2  # Swift 블록 주석은 중첩된다
            while depth:
                if i >= n:
                    fail(src, start, "닫히지 않은 블록 주석")
                if src.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif src.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
        elif c == '"':
            if src.startswith('"""', i):
                fail(src, i, "여러 줄 문자열 리터럴은 지원하지 않음")
            pos = i
            i += 1
            buf = []
            while True:
                if i >= n or src[i] == "\n":
                    fail(src, pos, "닫히지 않은 문자열")
                ch = src[i]
                if ch == '"':
                    i += 1
                    break
                if ch != "\\":
                    buf.append(ch)
                    i += 1
                    continue
                i += 1
                e = src[i] if i < n else ""
                if e in SIMPLE_ESC:
                    buf.append(SIMPLE_ESC[e])
                    i += 1
                elif e == "u" and src.startswith("{", i + 1):
                    j = src.find("}", i)
                    try:
                        buf.append(chr(int(src[i + 2:j], 16)))
                    except ValueError:
                        fail(src, i, "잘못된 \\u{...} 이스케이프")
                    i = j + 1
                elif e == "(":
                    fail(src, i, "문자열 보간 \\(...) 은 지원하지 않음")
                else:
                    fail(src, i, f"알 수 없는 이스케이프 \\{e}")
            yield ("str", "".join(buf), pos)
        elif c in ":(),":
            yield (c, c, i)
            i += 1
        elif c == "]":
            yield ("]", c, i)
            return
        else:
            fail(src, i, f"예상하지 못한 문자 {c!r}")
    fail(src, start, "테이블 끝(`]`)을 찾지 못함")


def parse(src):
    marker = "static let table: [String: (String, String, String)] = ["
    m = src.find(marker)
    if m < 0:
        raise ParseError("Loc.table 선언을 찾지 못함")
    toks = list(tokenize(src, m + len(marker)))
    table, i = {}, 0

    def expect(kind):
        nonlocal i
        if i >= len(toks) or toks[i][0] != kind:
            pos = toks[min(i, len(toks) - 1)][2]
            got = toks[i][0] if i < len(toks) else "EOF"
            fail(src, pos, f"{kind!r} 가 와야 하는데 {got!r}")
        t = toks[i]
        i += 1
        return t

    while toks[i][0] != "]":
        key = expect("str")
        expect(":")
        expect("(")
        vals = []
        for k in range(3):
            vals.append(expect("str")[1])
            if k < 2:
                expect(",")
        if toks[i][0] == ",":  # 튜플 끝 쉼표 허용
            i += 1
        expect(")")
        if toks[i][0] == ",":
            i += 1
        elif toks[i][0] != "]":
            fail(src, toks[i][2], "항목 사이에 쉼표가 없음")
        if key[1] in table:
            fail(src, key[2], f"중복 키 {key[1]!r}")
        table[key[1]] = vals
    return table


def main():
    try:
        table = parse(SRC.read_text(encoding="utf-8"))
    except ParseError as e:
        print(f"오류: {e}", file=sys.stderr)
        return 1
    # Rust 버전 UI 글에는 줄표(U+2014)를 쓰지 않는다(사용자 규칙). 하이픈으로 바꾼다.
    def no_em_dash(v):
        return v.replace(" \u2014 ", " - ").replace("\u2014", "-")
    ordered = {k: [no_em_dash(x) for x in table[k]] for k in sorted(table)}
    OUT.write_text(
        json.dumps(ordered, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(f"키 {len(ordered)}개 -> {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
