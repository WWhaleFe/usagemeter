namespace UsageMeter.Core;

/// <summary>한/영/일 문자열 — macOS Localization.swift에서 시작 세트 이식.
/// (전체 테이블은 기능 이식 진도에 맞춰 확장한다.)</summary>
public static class Loc
{
    /// <summary>(ko, en, ja)</summary>
    private static readonly Dictionary<string, (string Ko, string En, string Ja)> Table = new()
    {
        ["app.name"] = ("UsageMeter", "UsageMeter", "UsageMeter"),
        ["menu.settings"] = ("설정…", "Settings…", "設定…"),
        ["menu.refresh"] = ("지금 새로고침", "Refresh now", "今すぐ更新"),
        ["menu.quit"] = ("종료", "Quit", "終了"),
        ["menu.login"] = ("로그인…", "Log in…", "ログイン…"),
        ["menu.logout"] = ("로그아웃", "Log out", "ログアウト"),
        ["menu.noAI"] = ("로그인된 AI 없음", "No AI signed in", "ログイン中のAIなし"),
        ["menu.line5h"] = ("5시간 잔여", "5-hour left", "5時間残り"),
        ["menu.lineWeekly"] = ("주간 잔여", "Weekly left", "週間残り"),
        ["info.opus"] = ("Opus 주간", "Opus weekly", "Opus週間"),
        ["acct.loggedIn"] = ("로그인됨", "Logged in", "ログイン済み"),
        ["acct.loggedOut"] = ("로그아웃 상태", "Logged out", "未ログイン"),
        ["zone.main"] = ("본문", "Main", "メイン"),
        ["zone.task"] = ("작업표시줄", "Taskbar", "タスクバー"),
        ["lay.zoneAll"] = ("전체", "All", "全体"),
        ["part.taskLine"] = ("작업표시줄 경계선", "Taskbar line", "タスクバー境界線"),
        ["lay.reverse"] = ("차감 방향 반전", "Reverse depletion direction", "減少方向を反転"),
        ["cap.upShort"] = ("위", "Up", "上"),
        ["cap.downShort"] = ("아래", "Down", "下"),
        ["side.up"] = ("곡률 위", "Top of curve", "カーブの上"),
        ["side.corner"] = ("모서리", "Corner", "角"),
        ["side.down"] = ("곡률 아래", "Bottom of curve", "カーブの下"),
        // 설정창.
        ["settings.title"] = ("UsageMeter 설정", "UsageMeter Settings", "UsageMeter 設定"),
        ["tab.login"] = ("로그인", "Login", "ログイン"),
        ["tab.display"] = ("표시", "Display", "表示"),
        ["tab.line"] = ("선", "Line", "ライン"),
        ["tab.partition"] = ("화면 분할", "Partition", "画面分割"),
        ["tab.layout"] = ("배치", "Layout", "配置"),
        ["tab.radius"] = ("모서리 곡률", "Corners", "角の丸み"),
        ["line.thickness"] = ("굵기", "Thickness", "太さ"),
        ["line.opacity"] = ("투명도", "Opacity", "不透明度"),
        ["line.track"] = ("트랙 배경 표시", "Show track", "トラック背景"),
        ["line.splitOverlap"] = ("겹치는 선 굵기 분할", "Split overlapping lines", "重なる線の太さを分割"),
        ["part.enable"] = ("작업표시줄 경계선 사용", "Enable taskbar line", "タスクバー境界線を使用"),
        ["part.height"] = ("작업표시줄 높이", "Taskbar height", "タスクバーの高さ"),
        ["lay.presets"] = ("모양 프리셋", "Shape presets", "形状プリセット"),
        ["lay.hCapsTitle"] = ("가로선 끝 모서리", "Horizontal line end corners", "横線端の角"),
        ["lay.anchor"] = ("% 차감 시작 위치", "Depletion start", "減少開始位置"),
        ["lay.anchorSide"] = ("차감 시작 지점", "Start point", "開始点"),
        ["lay.left"] = ("왼쪽", "Left", "左"),
        ["lay.right"] = ("오른쪽", "Right", "右"),
        ["lay.invalid"] = ("선이 이어져야 합니다", "Segments must connect", "線がつながっている必要があります"),
        ["seg.hTop"] = ("상단", "Top", "上"),
        ["seg.hTask"] = ("작업표시줄선", "Taskbar line", "タスクバー線"),
        ["seg.hBottom"] = ("하단", "Bottom", "下"),
        ["corner.tl"] = ("좌상", "Top-left", "左上"),
        ["corner.tr"] = ("우상", "Top-right", "右上"),
        ["corner.br"] = ("우하", "Bottom-right", "右下"),
        ["corner.bl"] = ("좌하", "Bottom-left", "左下"),
        ["radius.link"] = ("본문↔작업표시줄 곡률 연결", "Link main↔taskbar radii", "メイン↔タスクバー角を連動"),
        ["display.language"] = ("언어", "Language", "言語"),
        ["display.refresh"] = ("자동 갱신 주기(분)", "Auto-refresh (min)", "自動更新(分)"),
    };

    public static string Tr(string key, string lang) =>
        Table.TryGetValue(key, out var e)
            ? lang switch { "ko" => e.Ko, "ja" => e.Ja, _ => e.En }
            : key;
}
