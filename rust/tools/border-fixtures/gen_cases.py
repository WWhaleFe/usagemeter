#!/usr/bin/env python3
"""cases.json 생성기. 기준 이미지(Swift)와 Rust 테스트가 같은 JSON을 읽는다.

각 케이스의 settings는 SettingsSnapshot(Swift Codable과 같은 키)로 **전부 명시**한다.
기기(노치 맞춤 등)나 기본값에 기대지 않으므로 Swift·Rust 양쪽에서 같은 입력이 보장된다.
ais: 켜진(로그인한) AI 목록. ratio = 대표 잔여율, fiveHour/weekly/models = 세부 항목 원천.
실행: python3 gen_cases.py > cases.json
"""
import json
import sys

SCREEN = [600, 400]
CLAUDE = [0.85, 0.47, 0.34, 1.0]
GEMINI = [0.26, 0.52, 0.96, 1.0]
CODEX = [0.06, 0.64, 0.50, 1.0]


def radii(tl, tr, br, bl):
    return {"topLeft": tl, "topRight": tr, "bottomRight": br, "bottomLeft": bl}


def all_r(r):
    return radii(r, r, r, r)


def layout(segs, corner="topLeft", side="center", cw=True, caps=None):
    d = {"anchorCorner": corner, "anchorSide": side, "clockwise": cw, "segments": segs}
    if caps:
        d["hCaps"] = caps
    return d


LOOP4 = ["hTop", "hBottom", "lMain", "rMain"]

DEFAULT_LAYOUTS = {
    "claude": layout(["hTop"], "topLeft"),
    "gemini": layout(["hBottom"], "bottomLeft"),
    "codex": layout(["lMain"], "bottomLeft"),
}


def base():
    return {
        "thickness": 3.0,
        "colorRGBA": [0.30, 0.82, 0.45, 1.0],
        "edges": ["top", "right", "bottom", "left"],
        "cornerRadii": all_r(22.0),
        "showTrack": True,
        "notchEnabled": False,
        "notchWidth": 186.0,
        "notchHeight": 33.0,
        "notchRadii": {"outerLeft": 5.0, "outerRight": 5.0, "innerLeft": 12.0, "innerRight": 12.0},
        "anchorCorner": "topLeft",
        "anchorSide": "center",
        "anchorClockwise": True,
        "lineOpacity": 1.0,
        "fadeEnabled": True,
        "fadeFraction": 0.012,
        "providerColors": {"claude": CLAUDE, "gemini": GEMINI, "codex": CODEX},
        "providerLayouts": {k: dict(v) for k, v in DEFAULT_LAYOUTS.items()},
        "menuLineEnabled": False,
        "menuLineHeight": 25.0,
        "dockLineEnabled": False,
        "dockLineHeight": 70.0,
        "menuZoneRadii": all_r(22.0),
        "dockZoneRadii": all_r(12.0),
        "globalSegments": LOOP4,
        "splitOverlapLines": False,
        "noOverlapLines": False,
        "subItemSplitIDs": [],
        "subItemColors": {},
        "subItemBarHidden": [],
    }


CASES = []


def case(name, ais, layouts=None, screen=None, **over):
    s = base()
    s.update(over)
    if layouts:
        s["providerLayouts"] = {**s["providerLayouts"], **layouts}
    CASES.append({
        "name": name,
        "screen": screen or SCREEN,
        "settings": s,
        "ais": [a if isinstance(a, dict) else {"id": a[0], "ratio": a[1]} for a in ais],
    })


def one(ratio, segs, corner="topLeft", side="center", cw=True, caps=None, aid="claude"):
    """AI 하나만 켠 케이스용: (ais, layouts)."""
    return [(aid, ratio)], {aid: layout(segs, corner, side, cw, caps)}


# ---- 기본 배치 ----
case("default_3ai", [("claude", 0.8), ("gemini", 0.5), ("codex", 0.37)],
     cornerRadii=radii(22, 22, 0, 0), menuZoneRadii=radii(22, 22, 0, 0), notchEnabled=True)
case("default_3ai_no_notch", [("claude", 0.8), ("gemini", 0.5), ("codex", 0.37)],
     cornerRadii=radii(22, 22, 0, 0))

# ---- 고리: 시작 꼭짓점 x 방향("가로 먼저") ----
for corner, tag in [("topLeft", "TL"), ("topRight", "TR"), ("bottomRight", "BR"), ("bottomLeft", "BL")]:
    for cw in (True, False):
        a, l = one(0.37, LOOP4, corner, "center", cw)
        case(f"loop_{tag}_{'cw' if cw else 'ccw'}", a, l)

# ---- 앵커 side ----
for side in ("left", "right"):
    a, l = one(0.37, LOOP4, "topLeft", side, True)
    case(f"loop_TL_side_{side}", a, l)

# ---- 잔여율 0 / 1 ----
a, l = one(0.0, LOOP4)
case("ratio_0_loop", a, l)
a, l = one(1.0, LOOP4)
case("ratio_1_loop", a, l)

# ---- 열린 체인 + 캡 ----
a, l = one(0.62, ["hTop"], caps={"hTop.L": "down", "hTop.R": "down"})
case("chain_htop_cap_down_both", a, l)
a, l = one(0.62, ["hBottom"], caps={"hBottom.L": "up", "hBottom.R": "up"})
case("chain_hbottom_cap_up_both", a, l)
a, l = one(0.62, ["hBottom"], "bottomLeft", caps={"hBottom.R": "up"}, cw=False)
case("chain_hbottom_cap_up_right_rev", a, l)
a, l = one(0.5, ["lMain", "hBottom", "rMain"], "bottomLeft")
case("chain_U", a, l)
a, l = one(0.5, ["hTop", "rMain"], "topLeft", caps={"hTop.L": "down"})
case("chain_L_cap_down_left", a, l)
a, l = one(0.5, ["lMain"], "bottomLeft")
case("chain_vertical_left", a, l)

# ---- 영역 곡률 0 / 12 / 22 / 40 ----
for r in (0, 12, 22, 40):
    a, l = one(0.37, LOOP4)
    case(f"radius_{r}_loop", a, l, cornerRadii=all_r(float(r)))

# ---- 노치 ----
a, l = one(0.5, ["hTop"])
case("notch_on_htop", a, l, notchEnabled=True, cornerRadii=radii(22, 22, 0, 0))
case("notch_off_htop", a, l, notchEnabled=False, cornerRadii=radii(22, 22, 0, 0))
a, l = one(0.5, LOOP4)
case("notch_loop_radii_0", a, l, notchEnabled=True,
     notchRadii={"outerLeft": 0.0, "outerRight": 0.0, "innerLeft": 0.0, "innerRight": 0.0})
case("notch_loop_radii_big", a, l, notchEnabled=True, notchWidth=260.0, notchHeight=45.0,
     notchRadii={"outerLeft": 14.0, "outerRight": 14.0, "innerLeft": 24.0, "innerRight": 24.0})
a, l = one(0.37, LOOP4, "topRight", "center", False)
case("notch_loop_TR_ccw", a, l, notchEnabled=True)

# ---- 페이드 / 트랙 / 투명도 ----
a, l = one(0.37, LOOP4)
case("fade_off", a, l, fadeEnabled=False)
case("fade_big", a, l, fadeFraction=0.15)
case("track_off", a, l, showTrack=False)
case("line_opacity_0.5", [("claude", 0.8), ("gemini", 0.5), ("codex", 0.37)], lineOpacity=0.5,
     cornerRadii=radii(22, 22, 0, 0))
case("thick_6", a, l, thickness=6.0)

# ---- 메뉴바선 / Dock선 ----
a, l = one(0.5, ["hTop", "hBottom", "lMenuBar", "lMain", "rMenuBar", "rMain"])
case("menu_line_loop", a, l, menuLineEnabled=True)
a, l = one(0.5, ["hTop", "hMenu", "lMenuBar", "rMenuBar"])
case("menu_line_loop_small", a, l, menuLineEnabled=True)
a, l = one(0.5, ["hTop", "hBottom", "lMenuBar", "lMain", "lDock", "rMenuBar", "rMain", "rDock"])
case("menu_dock_loop_all", a, l, menuLineEnabled=True, dockLineEnabled=True)
a, l = one(0.5, ["hMenu", "hDock", "lMain", "rMain"])
case("menu_dock_inner_loop", a, l, menuLineEnabled=True, dockLineEnabled=True,
     menuZoneRadii=all_r(10.0), dockZoneRadii=all_r(30.0))

# ---- 스쿱(오목 필렛) ----
a, l = one(0.5, ["rMenuBar", "hMenu"], "topRight", caps={"hMenu.R": "down"})
case("scoop_chain_menu_right", a, l, menuLineEnabled=True)
a, l = one(0.5, ["lMenuBar", "hMenu"], "topLeft", caps={"hMenu.L": "down"}, cw=False)
case("scoop_chain_menu_left", a, l, menuLineEnabled=True)
SCOOP_LOOP = ["hMenu", "hBottom", "lMain", "rMain"]
SCOOP_CAPS = {"hMenu.L": "up", "hMenu.R": "up"}
for side in ("left", "center", "right"):
    a, l = one(0.45, SCOOP_LOOP, "topRight", side, True, SCOOP_CAPS)
    case(f"scoop_loop_TR_{side}", a, l, menuLineEnabled=True)
a, l = one(0.45, SCOOP_LOOP, "topLeft", "center", False, SCOOP_CAPS)
case("scoop_loop_TL_ccw", a, l, menuLineEnabled=True)

# ---- 겹침 분할 ----
two = [("claude", 0.7), ("gemini", 0.4)]
lay2 = {"claude": layout(LOOP4), "gemini": layout(["hTop"], "topLeft")}
case("split2_off", two, lay2)
case("split2_on", two, lay2, splitOverlapLines=True)
case("split2_on_notch", two, lay2, splitOverlapLines=True, notchEnabled=True)
three = [("claude", 0.8), ("gemini", 0.55), ("codex", 0.3)]
lay3 = {"claude": layout(LOOP4), "gemini": layout(LOOP4, "bottomRight", "center", False),
        "codex": layout(["hTop", "rMain"], "topLeft")}
case("split3_off", three, lay3)
case("split3_on", three, lay3, splitOverlapLines=True, thickness=6.0)
case("split3_on_notch", three, lay3, splitOverlapLines=True, notchEnabled=True, thickness=6.0)

# ---- endOverride 반쪽 아크 (두 AI가 모서리를 반씩) ----
case("end_override_half", [("claude", 0.6), ("gemini", 0.6)],
     {"claude": layout(["hTop"], "topLeft", caps={"hTop.R": "down"}),
      "gemini": layout(["rMain"], "topRight")},
     cornerRadii=radii(22, 30, 0, 0))
case("end_override_half_bottom", [("claude", 0.7), ("gemini", 0.7)],
     {"claude": layout(["hBottom"], "bottomLeft", caps={"hBottom.L": "up"}),
      "gemini": layout(["lMain"], "bottomLeft")},
     cornerRadii=radii(0, 0, 30, 30))

# ---- 세부 항목 분할 띠 ----
sub = {"id": "claude", "ratio": 0.8, "fiveHour": 0.8, "weekly": 0.5,
       "models": [{"label": "Fable", "ratio": 0.2, "fiveHour": False}]}
case("subitems_claude", [sub], {"claude": layout(LOOP4)}, subItemSplitIDs=["claude"])
case("subitems_claude_hidden_color", [sub], {"claude": layout(LOOP4)}, subItemSplitIDs=["claude"],
     subItemBarHidden=["claude/weekly"], subItemColors={"claude/model:Fable:weekly": [0.9, 0.1, 0.1, 1.0]})
sub2 = {"id": "gemini", "ratio": 0.6, "fiveHour": 0.6, "weekly": 0.35}
case("subitems_two_ais", [sub, sub2], {"claude": layout(LOOP4), "gemini": layout(["hTop"])},
     subItemSplitIDs=["claude", "gemini"], splitOverlapLines=True)

json.dump(CASES, sys.stdout, ensure_ascii=False, indent=1)
print()
