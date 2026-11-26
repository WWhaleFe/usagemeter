#!/usr/bin/env python3
"""앱 아이콘을 코드로 그려 crates/um-app/icons 의 모든 크기를 다시 만든다.

사용: python3 rust/tools/draw-icon.py
필요: rsvg-convert(brew install librsvg), iconutil(macOS 기본)

스타일(MDemo 아이콘과 같은 틀): 연회색 바탕, 흰 카드와 넓고 옅은 그림자, 먹색 한 가지, 형광 라임 점 하나.
- 앞 카드 = 화면. 위와 오른쪽 가장자리를 따라가는 굵은 먹색 선 = UsageMeter가 화면 테두리에 그리는 사용량 선.
- 라임 점 = 선이 끝난 자리, 지금 남은 양.
- 뒤 카드(사선 해칭 + 점선 테두리) = 다른 모니터·다른 AI.
그림 전체를 macOS(Big Sur 이후) 아이콘 틀처럼 투명 여백 안의 둥근 사각형 타일에 넣는다.
"""
import shutil
import struct
import subprocess
import tempfile
from pathlib import Path

ICONS = Path(__file__).resolve().parents[1] / "crates/um-app/icons"
INK, LIME = "#1B1C1F", "#D4F53C"

# 1024 기준 안쪽 그림(앞 카드 194..734, 뒤 카드 330..830).
_hatch = "".join(f'<line x1="{x}" y1="670" x2="{x + 500}" y2="170"/>' for x in range(-170, 830, 40))
ART = f"""
<defs>
  <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#F0F1F3"/><stop offset="1" stop-color="#E4E6EA"/></linearGradient>
  <filter id="sh" x="-40%" y="-40%" width="180%" height="180%">
    <feGaussianBlur in="SourceAlpha" stdDeviation="55" result="b1"/><feOffset in="b1" dy="36" result="o1"/>
    <feFlood flood-color="#2A3140" flood-opacity="0.18"/><feComposite in2="o1" operator="in" result="c1"/>
    <feGaussianBlur in="SourceAlpha" stdDeviation="7" result="b2"/><feOffset in="b2" dy="6" result="o2"/>
    <feFlood flood-color="#2A3140" flood-opacity="0.08"/><feComposite in2="o2" operator="in" result="c2"/>
    <feMerge><feMergeNode in="c1"/><feMergeNode in="c2"/><feMergeNode in="SourceGraphic"/></feMerge>
  </filter>
  <clipPath id="bc"><rect x="336" y="176" width="488" height="488" rx="86"/></clipPath>
  <clipPath id="tile"><rect width="1024" height="1024" rx="230"/></clipPath>
</defs>
<g clip-path="url(#tile)">
  <rect width="1024" height="1024" fill="url(#bg)"/>
  <g clip-path="url(#bc)" stroke="#CDD0D6" stroke-width="10">{_hatch}</g>
  <rect x="334" y="174" width="492" height="492" rx="88" fill="none" stroke="#A4A9B1" stroke-width="8" stroke-linecap="round" stroke-dasharray="16 20"/>
  <rect x="194" y="304" width="540" height="540" rx="104" fill="#FBFBFC" filter="url(#sh)"/>
  <path d="M290,410 H566 Q638,410 638,482 V588" fill="none" stroke="{INK}" stroke-width="44" stroke-linecap="round"/>
  <circle cx="638" cy="676" r="34" fill="{LIME}"/>
</g>"""

# macOS 아이콘 틀: 1024 캔버스에 824 타일(여백 100).
SVG = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
<defs><filter id="tsh" x="-10%" y="-10%" width="120%" height="125%">
  <feGaussianBlur in="SourceAlpha" stdDeviation="10" result="b"/><feOffset in="b" dy="10" result="o"/>
  <feFlood flood-color="#000" flood-opacity="0.22"/><feComposite in2="o" operator="in" result="s"/>
  <feMerge><feMergeNode in="s"/><feMergeNode in="SourceGraphic"/></feMerge></filter></defs>
<g filter="url(#tsh)"><svg x="100" y="100" width="824" height="824" viewBox="0 0 1024 1024">{ART}</svg></g>
</svg>"""


def render(svg: Path, size: int, out: Path) -> None:
    subprocess.run(["rsvg-convert", "-w", str(size), "-h", str(size), str(svg), "-o", str(out)], check=True)


def write_ico(pngs: list[tuple[int, bytes]], out: Path) -> None:
    """PNG 항목으로 된 .ico (Windows Vista 이후 지원)."""
    head = struct.pack("<HHH", 0, 1, len(pngs))
    off = 6 + 16 * len(pngs)
    entries, data = b"", b""
    for size, png in pngs:
        s = 0 if size >= 256 else size
        entries += struct.pack("<BBBBHHII", s, s, 0, 0, 1, 32, len(png), off + len(data))
        data += png
    out.write_bytes(head + entries + data)


def main() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        svg = tmp / "icon.svg"
        svg.write_text(SVG)
        (ICONS / "icon.svg").write_text(SVG)

        # tauri.conf.json 과 MSIX(Assets/) 가 쓰는 PNG 들.
        sizes = {
            "icon.png": 512, "32x32.png": 32, "64x64.png": 64, "128x128.png": 128, "128x128@2x.png": 256,
            "Square30x30Logo.png": 30, "Square44x44Logo.png": 44, "Square71x71Logo.png": 71,
            "Square89x89Logo.png": 89, "Square107x107Logo.png": 107, "Square142x142Logo.png": 142,
            "Square150x150Logo.png": 150, "Square284x284Logo.png": 284, "Square310x310Logo.png": 310,
            "StoreLogo.png": 50,
        }
        for name, size in sizes.items():
            render(svg, size, ICONS / name)

        # macOS .icns
        iconset = tmp / "icon.iconset"
        iconset.mkdir()
        for base in (16, 32, 128, 256, 512):
            render(svg, base, iconset / f"icon_{base}x{base}.png")
            render(svg, base * 2, iconset / f"icon_{base}x{base}@2x.png")
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(ICONS / "icon.icns")], check=True)

        # Windows .ico
        pngs = []
        for size in (16, 24, 32, 48, 64, 256):
            p = tmp / f"ico{size}.png"
            render(svg, size, p)
            pngs.append((size, p.read_bytes()))
        write_ico(pngs, ICONS / "icon.ico")
    print(f"아이콘을 다시 만들었습니다: {ICONS}")


if __name__ == "__main__":
    if not shutil.which("rsvg-convert"):
        raise SystemExit("rsvg-convert가 필요합니다: brew install librsvg")
    main()
