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

        // 외형(라이트/다크)
        "appearance.title":  ("외형", "Appearance", "外観"),
        "appearance.system": ("기기 설정 따름", "Follow device", "デバイス設定に従う"),
        "appearance.light":  ("라이트", "Light", "ライト"),
        "appearance.dark":   ("다크", "Dark", "ダーク"),

        // AI 표시 순서
        "sec.providerOrder": ("AI 표시 순서", "AI Display Order", "AI表示順"),
        "order.desc":        ("드롭다운·호버에서 위에서부터 이 순서로 표시됩니다.", "Shown top-to-bottom in this order in the dropdown and hover.", "ドロップダウン・ホバーでこの順に上から表示されます。"),
        "menu.order":        ("AI 표시 순서", "AI display order", "AI表示順"),
        "order.up":          ("위로", "Move up", "上へ"),
        "order.down":        ("아래로", "Move down", "下へ"),

        // 후원
        "tab.support":       ("개발자에게", "To the developer", "開発者へ"),
        "sec.support":       ("개발자에게", "To the developer", "開発者へ"),
        // 리포트(문의/버그/제안)
        "report.title":      ("리포트 보내기", "Send a report", "レポートを送る"),
        "report.desc":       ("버그·오류·기능 제안 등 무엇이든 편하게 보내주세요. 아래 주소로 메일이 전송됩니다.", "Bugs, errors, feature ideas — anything is welcome. Mail is sent to the address below.", "バグ・不具合・機能のご提案など、お気軽にどうぞ。下記アドレスにメールが届きます。"),
        "report.openMail":   ("메일앱 실행", "Open mail app", "メールアプリを開く"),
        "report.copyEmail":  ("메일 주소 복사", "Copy address", "アドレスをコピー"),
        "report.copied":     ("복사됨", "Copied", "コピーしました"),
        "report.mailSubject":("UsageMeter 리포트", "UsageMeter report", "UsageMeter レポート"),
        // 후원
        "support.moreTitle": ("개발 후원", "Support development", "開発を支援"),
        "support.desc":      ("이 앱이 도움이 되셨다면 개발을 응원해 주세요. 아래 방법은 모두 수수료 없이 전달됩니다.", "If this app helps you, consider supporting its development. Both options below are fee-free.", "このアプリが役立ったら開発の応援をご検討ください。以下はどちらも手数料なしで届きます。"),
        "support.sponsors":  ("GitHub Sponsors 열기", "Open GitHub Sponsors", "GitHub Sponsorsを開く"),
        "support.kakaoTitle":("카카오페이로 커피 한 잔", "Buy me a coffee (KakaoPay)", "コーヒーを一杯（KakaoPay）"),
        "support.kakaoScan": ("아래 QR을 휴대폰 카카오페이로 스캔하세요.", "Scan the QR below with KakaoPay on your phone.", "下のQRをスマホのKakaoPayでスキャンしてください。"),
        "support.kakaoOpen": ("카카오페이 링크 열기", "Open KakaoPay link", "KakaoPayリンクを開く"),
        "support.thanks":    ("감사합니다! 🙇", "Thank you! 🙇", "ありがとうございます！🙇"),

        // 호버 정보 표시 토글
        "hover.enable":      ("테두리에 마우스 올리면 정보 표시", "Show info on border hover", "枠にマウスを乗せると情報を表示"),
        "hover.enableDesc":  ("화면 테두리의 색 띠 위에 커서를 올리면 사용량 정보를 띄웁니다.", "Shows usage info when you hover over the colored bands on screen edges.", "画面端の色帯にカーソルを乗せると使用量情報を表示します。"),
        "chartHover.enable": ("미니 차트에 마우스 올리면 값 표시", "Show values on mini-chart hover", "ミニチャートにマウスで値を表示"),

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
        "acct.geminiExperimental": ("· 실험적 — /usage 화면을 읽어오는 방식이라 구글 UI가 바뀌면 조회가 깨질 수 있어요",
                                    "· Experimental — reads the /usage page, so it may break if Google changes its UI",
                                    "· 実験的 — /usage画面を読み取る方式のため、GoogleのUI変更で取得が壊れる場合があります"),

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
        "disp.percentDesc": ("선택한 AI의 잔여 %가 각자 색으로 표시됩니다. 표시 순서는 'AI 표시 순서' 설정을 따릅니다.",
                             "Selected AIs' remaining % are shown in their colors. Order follows the 'AI Display Order' setting.",
                             "選択したAIの残量%を各色で表示します。順序は「AI表示順」設定に従います。"),
        "info.5h":      ("5시간 잔여", "5-hour remaining", "5時間の残量"),
        "info.weekly":  ("주간 잔여", "Weekly remaining", "週間の残量"),
        "info.opus":    ("모델별 주간 잔여 (예: Fable)", "Per-model weekly (e.g. Fable)", "モデル別の週間残量（例: Fable）"),
        "info.reset":   ("리셋 시각", "Reset time", "リセット時刻"),
        "info.countdown": ("리셋까지 남은 시간", "Time until reset", "リセットまでの時間"),
        "info.pace":    ("소진 예측", "Depletion forecast", "消費予測"),
        "info.chart":   ("미니 차트", "Mini chart", "ミニチャート"),
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
        "chart.titleFmt": ("최근 %d시간 잔여율(%)", "Remaining % (last %dh)", "残量%（直近%d時間）"),
        "chart.range":    ("미니 차트 기간", "Mini chart range", "ミニチャート期間"),
        "chart.hoursFmt": ("%d시간", "%d hours", "%d時間"),
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
        "lay.clockwise":   ("반시계방향으로 차감", "Deplete counterclockwise", "反時計回りに減少"),
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
        "reason.parseFailed": ("화면을 읽지 못함 — 서비스 UI 변경 가능성", "couldn't read the page — service UI may have changed", "画面を読み取れません — サービスUI変更の可能性"),
        "reason.serverError": ("서버 응답 오류", "server error", "サーバー応答エラー"),
        "reason.temporary":   ("일시 오류 — 잠시 후 자동 재시도", "temporary error — will retry", "一時的なエラー — 後で自動再試行"),

        // 메뉴바 드롭다운
        "menu.noAI":        ("로그인된 AI 없음 — 아래에서 로그인하세요", "No AI logged in — log in below", "ログイン済みAIなし — 下からログイン"),
        "menu.experimental": (" (실험적)", " (experimental)", "（実験的）"),
        "menu.line5h":      ("5시간 잔여", "5h remaining", "5時間残量"),
        "menu.lineWeekly":  ("주간 잔여", "Weekly remaining", "週間残量"),
        "menu.lineReset":   ("리셋", "reset", "リセット"),
        "menu.lineUpdated": ("마지막 갱신", "Last updated", "最終更新"),
        "menu.pickInfo":    ("표시할 정보를 '표시 정보'에서 선택하세요", "Pick items from 'Show info'", "「表示情報」から表示項目を選択"),
        "menu.analytics":   ("분석 보기", "View Analytics", "分析を見る"),
        "menu.showInfo":    ("표시 정보", "Show info", "表示情報"),

        // 분석 대시보드
        "analytics.title":       ("사용량 분석", "Usage Analytics", "使用量の分析"),
        "analytics.range":       ("기간", "Range", "期間"),
        "range.day1":            ("1일", "1 day", "1日"),
        "range.day3":            ("3일", "3 days", "3日"),
        "range.week":            ("주간", "Week", "週間"),
        "range.month":           ("한달", "Month", "1か月"),
        "analytics.byHour":      ("시간대별 사용 패턴", "Usage by Hour of Day", "時間帯別の使用パターン"),
        "analytics.byHourDesc":  ("하루 중 언제 AI를 주로 쓰는지 — 소비량을 시각별로 합산했습니다.", "When during the day you use AI most — consumption summed by hour.", "1日のいつAIをよく使うか — 消費量を時刻ごとに合計。"),
        "analytics.overTime":    ("기간별 소비 추이", "Consumption Over Time", "期間別の消費推移"),
        "analytics.overTimeDesc":("기간 동안의 소비량 변화입니다.", "Consumption across the selected range.", "選択期間における消費量の推移。"),
        "analytics.consumption": ("소비량", "Consumption", "消費量"),
        "analytics.hour":        ("시각", "Hour", "時刻"),
        "analytics.hourFmt":     ("%d시", "%d:00", "%d時"),
        "analytics.noData":      ("아직 표시할 데이터가 없어요.\n앱이 사용량을 수집하면서 점점 채워집니다.", "No data to show yet.\nIt fills in as the app collects usage over time.", "表示できるデータがまだありません。\nアプリが使用量を収集するにつれて増えていきます。"),
        "analytics.since":       ("수집 시작: %@", "Collecting since: %@", "収集開始: %@"),
        "analytics.busiest":     ("가장 활발한 시간대", "Busiest hour", "最も活発な時間帯"),
        "analytics.total":       ("누적 소비", "Total consumed", "累計消費"),
        "analytics.dailyAvg":    ("일 평균 소비", "Daily average", "1日平均"),
        "analytics.unitDesc":    ("5시간 한도 기준 회분", "in 5-hour-limit units", "5時間上限の回分"),
        "analytics.timesFmt":    ("%@회분", "%@×", "%@回分"),
        "analytics.byWeekday":   ("요일별 사용 패턴", "Usage by Day of Week", "曜日別の使用パターン"),
        "analytics.byWeekdayDesc":("어느 요일에 많이 쓰는지 — 소비량을 요일별로 합산했습니다.", "Which days you use AI most — consumption summed by weekday.", "どの曜日によく使うか — 消費量を曜日ごとに合計。"),
        "analytics.allAI":       ("전체", "All", "すべて"),
        "analytics.noDataFlash": ("%@: 아직 데이터 없음 — 아래 '주의' 내용을 참고하세요", "%@: no data yet — see the “Notes” below", "%@: データがまだありません — 下の「注意」をご覧ください"),
        "analytics.share":       ("AI별 소비 비중", "Consumption Share by AI", "AI別の消費割合"),
        "analytics.shareDesc":   ("기간 동안 AI별로 얼마나 썼는지 비율입니다.", "Share of consumption by AI over the period.", "期間中のAI別消費の割合です。"),
        "analytics.aboutTitle":  ("이 창에 대하여", "About this view", "この画面について"),
        "analytics.about":       ("앱이 기록한 사용 이력을 바탕으로, AI별 소비량(잔여율이 줄어든 만큼)을 시간대·기간별로 보여줍니다. 하루 중 언제 많이 쓰는지, 기간 동안 얼마나 썼는지 한눈에 파악할 수 있어요.",
                                  "Built from the usage history the app records, this shows each AI's consumption (how much the remaining ratio dropped) by hour and over time — so you can see when you use AI most and how much across a period.",
                                  "アプリが記録した使用履歴をもとに、AIごとの消費量（残量が減った分）を時間帯・期間別に表示します。1日のいつ多く使うか、期間中どれだけ使ったかが一目で分かります。"),
        "analytics.cautionTitle":("주의", "Notes", "注意"),
        "analytics.c1":          ("로그인되어 사용량이 조회되는 AI만 집계됩니다.", "Only AIs that are logged in and returning usage are counted.", "ログインして使用量が取得できるAIのみ集計されます。"),
        "analytics.c2":          ("값은 5시간 한도 기준 상대 소비량이며, 리셋(잔여 증가) 구간은 제외합니다.", "Values are relative to the 5-hour limit; reset periods (remaining goes up) are excluded.", "値は5時間上限を基準とした相対消費量で、リセット（残量増加）区間は除外します。"),
        "analytics.c3":          ("과거는 소급되지 않고, 지금부터 최대 35일까지 누적됩니다.", "History isn't backfilled — it accumulates from now, up to 35 days.", "過去には遡らず、今から最大35日まで蓄積されます。"),
        "analytics.c4":          ("자동 갱신 주기보다 짧은 사용은 뭉쳐서 반영될 수 있습니다.", "Usage shorter than the refresh interval may be grouped together.", "自動更新間隔より短い使用はまとめて反映されることがあります。"),
        "menu.aiLoginout":  ("AI 로그인 / 로그아웃", "AI Login / Logout", "AI ログイン / ログアウト"),
        "menu.refresh":     ("사용량 새로고침", "Refresh usage", "使用量を更新"),
        "menu.settings":    ("앱 설정", "Settings", "設定"),
        "menu.presets":     ("프리셋", "Presets", "プリセット"),
        "menu.hideBorder":      ("테두리 숨기기", "Hide border", "枠を隠す"),
        "menu.deviceFit":       ("이 기기에 맞춤", "Fit to this device", "この端末に合わせる"),
        "menu.reportSubmenu":   ("개발자에게 리포트", "Report to developer", "開発者へレポート"),
        "menu.reportTitle":     ("개발자의 메일 주소", "Developer's email", "開発者のメールアドレス"),
        "menu.reportMail":      ("메일 앱으로", "Open in mail app", "メールアプリで"),
        "menu.reportCopy":      ("메일주소 복사", "Copy address", "アドレスをコピー"),
        "menu.support":         ("후원", "Support", "支援"),
        "menu.supportSponsors": ("GitHub Sponsors", "GitHub Sponsors", "GitHub Sponsors"),
        "menu.supportKakao":    ("카카오페이로 커피 한 잔", "Buy me a coffee (KakaoPay)", "コーヒーを一杯（KakaoPay）"),
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
