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
        "tab.login":   ("로그인", "Login", "ログイン"),
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
        "login.autostart": ("Mac 로그인 시 자동 실행", "Launch at login", "Macログイン時に自動起動"),
        "login.autostartNote": ("패키징된 앱(.app)에서만 적용됩니다.", "Applies only to the packaged app (.app).", "パッケージ版アプリ（.app）でのみ有効です。"),

        // 화면 분할(파티션) + 배치 미리보기
        "sec.preview":     ("미리보기", "Preview", "プレビュー"),
        "sec.partition":   ("화면 분할 (가로 경계선)", "Screen Partitions (Horizontal Lines)", "画面分割（横の境界線）"),
        "part.menuLine":   ("메뉴바 경계선", "Menu bar line", "メニューバー境界線"),
        "part.menuHeight": ("메뉴바 경계 위치 (위에서, pt)", "Menu bar line offset (from top, pt)", "メニューバー境界の位置（上から、pt）"),
        "part.dockLine":   ("Dock 경계선", "Dock line", "Dock境界線"),
        "part.dockHeight": ("Dock 경계 높이 (아래에서, pt)", "Dock line height (from bottom, pt)", "Dock境界の高さ（下から、pt）"),
        "part.desc": ("경계선을 켜면 아래 배치에서 각 AI가 감쌀 영역을 고를 수 있습니다. 붙은 영역을 함께 선택하면 합쳐진 둘레로 그려집니다. 모서리 곡률은 '모서리 곡률' 탭(전역) 또는 AI별 '곡률 개별 설정'에서 꼭짓점마다 조절하며, 경계선 끝에도 그대로 적용됩니다(위 영역=위로, 아래 영역=아래로 둥글게).",
                      "With a line enabled, pick which zones each AI wraps below. Selecting adjacent zones merges them into one outline. Corner radii are set per-vertex in the Corners tab (global) or per-AI via 'Custom corner radii' — they also apply to line ends (upper zone = curves up, lower = down).",
                      "境界線を有効にすると、下の配置で各AIが囲む領域を選べます。隣接する領域を同時に選ぶと結合した外周になります。角の丸みは「角の丸み」タブ（全体）またはAIごとの「丸み個別設定」で頂点ごとに調整でき、境界線の端にも適用されます（上の領域=上へ、下=下へ）。"),
        "sec.zoneRadii": ("모서리 곡률 (영역별)", "Corner Radius (per zone)", "角の丸み（領域別）"),
        "lay.capNeedRadius": ("모서리 포함은 해당 꼭짓점의 곡률이 0보다 커야 켤 수 있습니다.",
                              "Corner include requires that vertex's radius to be greater than 0.",
                              "角を含めるには、その頂点の丸みが0より大きい必要があります。"),
        "zone.menuBar": ("메뉴바", "Menu bar", "メニューバー"),
        "zone.main":    ("본문", "Main", "メイン"),
        "zone.dock":    ("Dock", "Dock", "Dock"),
        "lay.zones":    ("영역", "Zones", "領域"),

        // 세그먼트(가로 4 + 세로 좌·우 각 3)
        "seg.hTop":     ("상단 ─", "Top ─", "上 ─"),
        "seg.hMenu":    ("메뉴바선 ─", "Menu line ─", "メニュー線 ─"),
        "seg.hDock":    ("Dock선 ─", "Dock line ─", "Dock線 ─"),
        "seg.hBottom":  ("하단 ─", "Bottom ─", "下 ─"),
        "seg.lMenuBar": ("좌·메뉴바 │", "L·menu │", "左・メニュー │"),
        "seg.lMain":    ("좌·본문 │", "L·main │", "左・メイン │"),
        "seg.lDock":    ("좌·Dock │", "L·dock │", "左・Dock │"),
        "seg.rMenuBar": ("우·메뉴바 │", "R·menu │", "右・メニュー │"),
        "seg.rMain":    ("우·본문 │", "R·main │", "右・メイン │"),
        "seg.rDock":    ("우·Dock │", "R·dock │", "右・Dock │"),
        "lay.quickZone": ("영역 빠른 선택", "Quick zone select", "領域クイック選択"),
        "lay.zoneAll":   ("전체", "All", "全体"),
        "lay.presets":     ("모양 프리셋", "Shape presets", "形プリセット"),
        "preset.full":     ("전체 테두리", "Full border", "全体枠"),
        "preset.menuWrap": ("메뉴바 감싸개", "Menu bar wrap", "メニューバー包み"),
        "preset.mainWrap": ("본문 감싸기", "Main wrap", "メイン包み"),
        "preset.dockWrap": ("Dock 감싸개", "Dock wrap", "Dock包み"),
        "lay.capStart":  ("시작 끝 모서리", "Start-end corner", "始端の角"),
        "lay.capEnd":    ("반대 끝 모서리", "Far-end corner", "終端の角"),
        "cap.none":      ("없음", "None", "なし"),
        "cap.up":        ("위로 둥글게", "Curve up", "上へ丸く"),
        "cap.down":      ("아래로 둥글게", "Curve down", "下へ丸く"),
        "cap.include":   ("모서리 포함", "Include corner", "角を含める"),
        "cap.upShort":   ("위", "Up", "上"),
        "cap.downShort": ("아래", "Down", "下"),
        "lay.hCapsTitle": ("가로선 끝 모서리 — 양 끝을 위/아래로 둥글게 꺾기", "Horizontal line ends — curve each end up/down", "横線の端の角 — 両端を上/下へ丸く曲げる"),
        "cap.leftEnd":  ("왼쪽", "Left", "左"),
        "cap.rightEnd": ("오른쪽", "Right", "右"),
        "cap.upHelp":   ("이 끝의 모서리를 위쪽으로 둥글게 꺾습니다", "Curve this end's corner upward", "この端の角を上へ丸く曲げます"),
        "cap.downHelp": ("이 끝의 모서리를 아래쪽으로 둥글게 꺾습니다", "Curve this end's corner downward", "この端の角を下へ丸く曲げます"),
        "lay.reverse":   ("차감 방향 반전", "Reverse depletion direction", "減少方向を反転"),
        "lay.anchorSide": ("차감 시작 지점 (모서리 곡선 기준)", "Depletion start point (on corner curve)", "減少開始地点（角カーブ基準）"),
        "side.up":   ("곡률 위", "Top of curve", "カーブの上"),
        "side.corner": ("모서리", "Corner", "角"),
        "side.down": ("곡률 아래", "Bottom of curve", "カーブの下"),
        "lay.overlapWarn": ("겹치는 선입니다", "Overlapping line", "重なっている線です"),
        "tab.partition": ("화면 분할", "Partitions", "画面分割"),
        "sec.overlap": ("겹침 표시", "Overlap Display", "重なりの表示"),
        "overlap.layerDesc": ("완전히 같은 모양끼리 겹치면 잔여율이 가장 적은(급한) 띠가 맨 위에 표시됩니다. 모양이 다른 부분 겹침은 그 구간만 굵기를 나눠 나란히 표시합니다.",
                              "Identical shapes stack with the lowest-remaining (most urgent) band on top. Partial overlaps between different shapes split the thickness side by side in the shared span only.",
                              "完全に同じ形が重なる場合は残量が最も少ない（急ぎ）バンドが最前面に表示されます。形が異なる部分的な重なりは、その区間だけ太さを分けて並べて表示します。"),
        "lay.splitOverlap": ("겹침 구간 굵기 분할 (동일 모양 포함)", "Split thickness in overlaps (incl. identical shapes)", "重なり区間の太さを分割（同一形状も）"),
        "corner.linkMenuMain": ("메뉴바 ↔ 본문 곡률 연결 (맞닿은 꼭짓점 함께 조절)", "Link menu bar ↔ main radii (adjust touching corners together)", "メニューバー ↔ メイン丸み連動（接する角を一緒に調整）"),
        "corner.linkMainDock": ("본문 ↔ Dock 곡률 연결 (맞닿은 꼭짓점 함께 조절)", "Link main ↔ Dock radii (adjust touching corners together)", "メイン ↔ Dock丸み連動（接する角を一緒に調整）"),
        "overlap.splitDesc": ("켜면 완전히 같은 모양끼리 겹칠 때도 굵기를 나눠 모두 보이게 합니다.",
                              "When on, even identical shapes split the thickness so every band stays visible.",
                              "オンにすると、完全に同じ形の重なりでも太さを分けてすべて表示します。"),
        "lay.dblHint": ("더블 클릭: 그 선 하나만 선택", "Double-click: select only that line", "ダブルクリック：その線だけ選択"),
        "part.dragAdjust": ("화면에서 드래그로 조정…", "Adjust by dragging on screen…", "画面上でドラッグして調整…"),
        "part.dragDone": ("완료", "Done", "完了"),
        "lay.noOverlap": ("겹치는 선 없음 (AI끼리 같은 선 선택 불가)", "No overlapping lines (AIs can't share a segment)", "線の重なりなし（AI間で同じ線は選択不可）"),

        // 로그인 탭 (AI 계정)
        "sec.accounts":   ("AI 계정", "AI Accounts", "AIアカウント"),
        "sec.loginOpts":  ("로그인 옵션", "Login Options", "ログインオプション"),
        "acct.loggedIn":  ("로그인됨", "Logged in", "ログイン済み"),
        "acct.loggedOut": ("로그아웃 상태", "Logged out", "未ログイン"),
        "acct.login":     ("로그인…", "Log in…", "ログイン…"),
        "acct.logout":    ("로그아웃", "Log out", "ログアウト"),

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
        "info.opus":    ("Opus 주간 잔여", "Opus weekly remaining", "Opus週間の残量"),
        "info.reset":   ("리셋 시각", "Reset time", "リセット時刻"),
        "info.countdown": ("리셋까지 남은 시간", "Time until reset", "リセットまでの時間"),
        "info.pace":    ("소진 예측", "Depletion forecast", "消費予測"),
        "info.chart":   ("24시간 미니 차트", "24-hour mini chart", "24時間ミニチャート"),
        "info.updated": ("마지막 갱신 시각", "Last updated", "最終更新時刻"),

        // 카운트다운/소진예측
        "cd.afterFmt":   ("%@ 후", "in %@", "%@後"),
        "cd.hour":       ("시간", "h", "時間"),
        "cd.min":        ("분", "m", "分"),
        "cd.now":        ("곧", "soon", "まもなく"),
        "pace.depleteFmt": ("%@ 후 소진 예상", "empty in %@", "%@後に枯渇予想"),
        "pace.warnReset":  ("· 리셋 전 ⚠", "· before reset ⚠", "· リセット前 ⚠"),
        "pace.okReset":    ("· 리셋 후 여유", "· after reset (safe)", "· リセット後（安全）"),

        // 미니 차트
        "chart.title":   ("최근 24시간 잔여율(%)", "Remaining % (last 24h)", "残量%（直近24時間）"),
        "chart.collecting": ("차트 데이터 수집 중…", "Collecting chart data…", "チャートデータ収集中…"),

        // 임계치 알림
        "sec.notify":    ("임계치 알림", "Threshold Alerts", "しきい値アラート"),
        "notify.enable": ("사용량 임계치 알림 켜기", "Enable usage threshold alerts", "使用量しきい値アラートを有効化"),
        "notify.thFmt":  ("%d% 사용 시 알림", "Alert at %d% used", "%d%使用時にアラート"),
        "notify.desc":   ("선택한 사용률에 도달하면 macOS 알림을 보냅니다 (리셋 주기당 1회).", "Sends a macOS notification when usage reaches the selected level (once per reset cycle).", "選択した使用率に達するとmacOS通知を送ります（リセット周期ごとに1回）。"),
        "notify.title":  ("AI 사용량 경고", "AI usage warning", "AI使用量の警告"),
        "notify.body":   ("%@ 사용량이 %d% 도달", "%@ usage reached %d%", "%@ の使用量が %d% に到達"),

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
        "lay.endFinish":   ("모서리 포함 (선 끝)", "Include corners (line ends)", "角を含める（線端）"),
        "lay.extendStart": ("% 차감 시작쪽 모서리 포함", "Include depletion-start corner", "%減少開始側の角を含める"),
        "lay.noCurve":     ("↳ 곡선을 이웃 변으로 안 뻗기", "↳ Don't extend curve to neighbor edge", "↳ 曲線を隣の辺へ伸ばさない"),
        "lay.extendEnd":   ("반대쪽 끝 모서리 포함", "Include far-end corner", "反対側の端の角を含める"),
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
        "notch.detect":       ("이 화면에서 자동 감지", "Auto-detect from this screen", "この画面から自動検出"),

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
