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
    };

    public static string Tr(string key, string lang) =>
        Table.TryGetValue(key, out var e)
            ? lang switch { "ko" => e.Ko, "ja" => e.Ja, _ => e.En }
            : key;
}
