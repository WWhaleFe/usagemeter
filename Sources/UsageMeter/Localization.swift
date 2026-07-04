import Foundation

/// 앱 UI 언어. 설정에서 전환하면 설정 창·메뉴 문자열이 이 언어로 바뀐다.
enum AppLanguage: String, CaseIterable, Codable {
    case korean = "ko"
    case english = "en"
    case japanese = "ja"

    /// 언어 선택 피커에 보일 이름(각 언어 고유 표기).
    var displayName: String {
        switch self {
        case .korean: return "한국어"
        case .english: return "English"
        case .japanese: return "日本語"
        }
    }
}

/// 문자열 로컬라이제이션. `Loc.tr(key, lang)` 로 (한/영/일) 중 하나를 반환.
enum Loc {
    static func tr(_ key: String, _ lang: AppLanguage) -> String {
        guard let e = table[key] else { return key }
        switch lang {
        case .korean:   return e.0
        case .english:  return e.1
        case .japanese: return e.2
        }
    }

    /// (한국어, English, 日本語)
    static let table: [String: (String, String, String)] = [
        // 언어
        "lang.title": ("언어", "Language", "言語"),

        // 자동 갱신 주기
        "tab.interval":        ("주기", "Interval", "更新間隔"),
        "sec.interval":        ("자동 갱신 주기", "Auto-refresh Interval", "自動更新の間隔"),
        "interval.minutesFmt": ("%d분", "%d min", "%d分"),
        "interval.preset":     ("갱신 주기", "Refresh interval", "更新間隔"),
        "interval.useCustom":  ("사용자 설정 (직접 분 입력)", "Custom (enter minutes)", "カスタム（分を直接入力）"),
        "interval.customMin":  ("직접 주기 (분)", "Custom interval (min)", "カスタム間隔（分）"),
        "interval.currentFmt": ("현재 %d분마다 자동으로 사용량을 갱신합니다.", "Auto-refreshes usage every %d minutes.", "%d分ごとに使用量を自動更新します。"),
        "menu.interval":       ("자동 갱신 주기", "Auto-refresh interval", "自動更新の間隔"),
        "menu.intervalCustom": ("사용자 설정…", "Custom…", "カスタム…"),

        // 탭
        "tab.presets": ("저장 불러오기", "Save & Load", "保存・読み込み"),
        "tab.display": ("표시", "Display", "表示"),
        "tab.line":    ("선", "Line", "線"),
        "tab.layout":  ("배치", "Layout", "配置"),
        "tab.corner":  ("모서리 곡률", "Corners", "角の丸み"),
        "tab.notch":   ("노치", "Notch", "ノッチ"),

        // 섹션 제목
        "sec.baseState":    ("기본 상태", "Default State", "既定の状態"),
        "sec.login":        ("로그인", "Login", "ログイン"),
        "sec.presets":      ("프리셋 (최대 %d개)", "Presets (max %d)", "プリセット（最大%d個）"),
        "sec.menuIcon":     ("메뉴바 아이콘", "Menu Bar Icon", "メニューバーアイコン"),
        "sec.menuPercent":  ("메뉴바 아이콘 옆 % 표시", "% Next to Menu Bar Icon", "メニューバーアイコン横の%表示"),
        "sec.dropdownInfo": ("드롭다운 표시 정보", "Dropdown Info", "ドロップダウン表示項目"),
        "sec.line":         ("선", "Line", "線"),
        "sec.aiColor":      ("AI 선 색", "AI Line Colors", "AI線の色"),
        "sec.layout":       ("배치 (변 · 방향)", "Layout (Edges · Direction)", "配置（辺・方向）"),
        "sec.corner":       ("모서리 곡률", "Corner Radius", "角の丸み"),
        "sec.notch":        ("노치 감싸기", "Notch Wrap", "ノッチ回避"),

        // 기본 상태
        "base.save":  ("현재 상태를 기본으로 저장", "Save current as default", "現在の状態を既定として保存"),
        "base.reset": ("기본 상태로 초기화", "Reset to default", "既定の状態に戻す"),

        // 로그인
        "login.keep": ("로그인 유지 (재실행해도 자동 로그인)", "Stay logged in (auto-login on restart)", "ログイン維持（再起動時も自動ログイン）"),

        // 프리셋
        "preset.newName": ("새 프리셋 이름", "New preset name", "新しいプリセット名"),
        "preset.save":    ("저장", "Save", "保存"),

        // 표시(메뉴바/드롭다운)
        "disp.iconBaseAI": ("아이콘 기준 AI", "Icon based on", "アイコン基準AI"),
        "disp.iconDesc":   ("메뉴바 링 아이콘이 이 AI의 잔여율·색으로 표시됩니다 (로그인 안 됐으면 자동).",
                            "The menu bar ring shows this AI's remaining ratio and color (auto if not logged in).",
                            "メニューバーのリングはこのAIの残量・色で表示されます（未ログイン時は自動）。"),
        "disp.showPercent": ("%@ % 표시", "Show %@ %", "%@ の%を表示"),
        "disp.firstAI":    ("먼저 표시할 AI", "AI shown first", "先に表示するAI"),
        "disp.percentDesc": ("선택한 AI의 잔여 %가 각자 색으로 표시됩니다. 순서는 둘 다 선택했을 때 적용됩니다.",
                             "Selected AIs' remaining % are shown in their colors. Order applies when both are selected.",
                             "選択したAIの残量%を各色で表示します。順序は両方選択時に適用されます。"),
        "info.5h":      ("5시간 잔여", "5-hour remaining", "5時間の残量"),
        "info.weekly":  ("주간 잔여", "Weekly remaining", "週間の残量"),
        "info.reset":   ("리셋 시각", "Reset time", "リセット時刻"),
        "info.updated": ("마지막 갱신 시각", "Last updated", "最終更新時刻"),

        // 선
        "line.thickness": ("굵기", "Thickness", "太さ"),
        "line.opacity":   ("투명도 (클수록 투명)", "Transparency (higher = clearer)", "透明度（大きいほど透明）"),
        "line.track":     ("트랙 배경 표시", "Show track background", "トラック背景を表示"),
        "line.fade":      ("끝부분 투명 그라데이션", "Fade at the end", "端のフェード"),
        "line.fadeLen":   ("그라데이션 길이(%)", "Fade length (%)", "フェード長さ(%)"),

        // AI 선 색
        "color.lineColor": ("%@ 선색", "%@ line", "%@ 線の色"),

        // 배치
        "lay.separate":   ("AI별로 변을 나눠 쓰기 (겹치지 않음)", "Separate edges per AI (no overlap)", "AIごとに辺を分ける（重ならない）"),
        "lay.overlapDesc": ("모든 AI가 같은 테두리에 겹쳐 표시됩니다 (잔여율 낮은 AI가 앞).",
                            "All AIs overlap on the same border (lower remaining in front).",
                            "すべてのAIが同じ枠に重なって表示されます（残量が少ないAIが前面）。"),
        "lay.endFinish":   ("선 끝 마감 (일부 변만 선택 시)", "Line end caps (partial edges)", "線端の仕上げ（一部の辺のみ）"),
        "lay.extendStart": ("% 차감 시작쪽 모서리 둥글게", "Round the depletion-start corner", "%減少開始側の角を丸く"),
        "lay.noCurve":     ("↳ 곡선을 이웃 변으로 안 뻗기", "↳ Don't extend curve to neighbor edge", "↳ 曲線を隣の辺へ伸ばさない"),
        "lay.extendEnd":   ("반대쪽 끝 모서리 둥글게", "Round the far-end corner", "反対側の端の角を丸く"),
        "lay.anchor":      ("% 차감 시작 위치", "Depletion start position", "%減少開始位置"),
        "lay.clockwise":   ("시계방향으로 차감", "Deplete clockwise", "時計回りに減少"),
        "lay.partialDir":  ("일부 변만: 차감 방향은 ‘% 차감 시작 위치’가 정합니다.",
                            "Partial edges: direction is set by ‘Depletion start position’.",
                            "一部の辺のみ：減少方向は「%減少開始位置」で決まります。"),

        // 변
        "edge.top":    ("상", "Top", "上"),
        "edge.bottom": ("하", "Bottom", "下"),
        "edge.left":   ("좌", "Left", "左"),
        "edge.right":  ("우", "Right", "右"),

        // 모서리
        "corner.topLeft":     ("좌상", "Top-L", "左上"),
        "corner.topRight":    ("우상", "Top-R", "右上"),
        "corner.bottomRight": ("우하", "Bottom-R", "右下"),
        "corner.bottomLeft":  ("좌하", "Bottom-L", "左下"),

        // 노치
        "notch.enable":       ("노치 감싸기 켜기", "Enable notch wrap", "ノッチ回避を有効化"),
        "notch.width":        ("너비", "Width", "幅"),
        "notch.height":       ("높이(깊이)", "Height (depth)", "高さ（深さ）"),
        "notch.cornerRadius": ("노치 모서리 곡률", "Notch corner radius", "ノッチ角の丸み"),
        "notch.outerLeft":    ("좌·바깥(위)", "Left·outer(top)", "左・外(上)"),
        "notch.innerLeft":    ("좌·안쪽(아래)", "Left·inner(bottom)", "左・内(下)"),
        "notch.innerRight":   ("우·안쪽(아래)", "Right·inner(bottom)", "右・内(下)"),
        "notch.outerRight":   ("우·바깥(위)", "Right·outer(top)", "右・外(上)"),

        // 테두리 호버 정보
        "hover.noAI":        ("로그인된 AI 없음 — 메뉴에서 로그인하세요", "No AI logged in — log in from the menu", "ログイン済みAIなし — メニューからログイン"),
        "hover.authExpired": ("로그인이 필요합니다", "Login required", "ログインが必要です"),
        "hover.unavailable": ("사용량을 못 읽음 (%@)", "Can't read usage (%@)", "使用量を取得できません（%@）"),

        // 메뉴바 드롭다운
        "menu.noAI":        ("로그인된 AI 없음 — 아래에서 로그인하세요", "No AI logged in — log in below", "ログイン済みAIなし — 下からログイン"),
        "menu.experimental": (" (실험적)", " (experimental)", "（実験的）"),
        "menu.line5h":      ("5시간 잔여", "5h remaining", "5時間残量"),
        "menu.lineWeekly":  ("주간 잔여", "Weekly remaining", "週間残量"),
        "menu.lineReset":   ("리셋", "reset", "リセット"),
        "menu.lineUpdated": ("마지막 갱신", "Last updated", "最終更新"),
        "menu.pickInfo":    ("표시할 정보를 '표시 정보'에서 선택하세요", "Pick items from 'Show info'", "「表示情報」から表示項目を選択"),
        "menu.showInfo":    ("표시 정보", "Show info", "表示情報"),
        "menu.aiLoginout":  ("AI 로그인 / 로그아웃", "AI Login / Logout", "AI ログイン / ログアウト"),
        "menu.refresh":     ("사용량 새로고침", "Refresh usage", "使用量を更新"),
        "menu.settings":    ("앱 설정", "Settings", "設定"),
        "menu.presets":     ("프리셋", "Presets", "プリセット"),
        "menu.quit":        ("종료", "Quit", "終了"),
        "menu.login":       ("  로그인…", "  Log in…", "  ログイン…"),
        "menu.logout":      ("  로그아웃", "  Log out", "  ログアウト"),
        "menu.noPreset":    ("저장된 프리셋 없음", "No saved presets", "保存済みプリセットなし"),
        "menu.loadPrefix":  ("불러오기: ", "Load: ", "読み込み："),
        "menu.deletePrefix": ("삭제: ", "Delete: ", "削除："),
        "menu.saveCurrent": ("현재 설정 저장…", "Save current…", "現在の設定を保存…"),
        "menu.savePresetTitle": ("현재 설정을 프리셋으로 저장", "Save current settings as preset", "現在の設定をプリセットとして保存"),
        "menu.savePresetInfo":  ("이름을 입력하세요 (최대 %d개).", "Enter a name (max %d).", "名前を入力してください（最大%d個）。"),
        "menu.presetDefaultName": ("프리셋", "Preset", "プリセット"),
        "menu.cancel":      ("취소", "Cancel", "キャンセル"),
    ]
}
