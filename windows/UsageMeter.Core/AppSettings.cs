using System.Text.Json;
using System.Text.Json.Serialization;

namespace UsageMeter.Core;

/// <summary>AI별 배치 — macOS ProviderLayout(세그먼트 모델) 이식.</summary>
public sealed class ProviderLayout
{
    public HashSet<SegPart> Segments { get; set; } = new() { SegPart.HTop };
    public BorderCorner AnchorCorner { get; set; } = BorderCorner.TopLeft;
    public AnchorSide AnchorSide { get; set; } = AnchorSide.Left;
    public bool Clockwise { get; set; } = true;
    /// <summary>가로선 좌/우 끝 캡. 키 = SegGraph.HCapKey.</summary>
    public Dictionary<string, SegCap> HCaps { get; set; } = new();
    public double[]? ColorRgb { get; set; }      // 사용자 지정 색(없으면 브랜드 기본)
}

/// <summary>앱 설정 — %APPDATA%\UsageMeter\settings.json 영속화(macOS OverlaySettings 이식).</summary>
public sealed class AppSettings
{
    public double Thickness { get; set; } = 2;
    public double LineOpacity { get; set; } = 1.0;
    public bool ShowTrack { get; set; } = true;
    public bool FadeEnabled { get; set; } = true;
    public double FadeFraction { get; set; } = 0.012;

    // 화면 분할(2분할): 작업표시줄 경계선 하나만.
    public bool TaskLineEnabled { get; set; }
    public double TaskLineHeight { get; set; } = 48;

    // 영역별 모서리 곡률.
    public Dictionary<BorderCorner, double> MainRadii { get; set; } = new()
    {
        [BorderCorner.TopLeft] = 8, [BorderCorner.TopRight] = 8,
        [BorderCorner.BottomLeft] = 8, [BorderCorner.BottomRight] = 8,
    };
    public Dictionary<BorderCorner, double> TaskRadii { get; set; } = new()
    {
        [BorderCorner.TopLeft] = 8, [BorderCorner.TopRight] = 8,
        [BorderCorner.BottomLeft] = 8, [BorderCorner.BottomRight] = 8,
    };
    /// <summary>본문↔작업표시줄 경계의 맞닿은 곡률 동기화.</summary>
    public bool LinkMainTaskRadii { get; set; } = true;

    // 겹침.
    public bool NoOverlapLines { get; set; }
    public bool SplitOverlapLines { get; set; }

    public Dictionary<string, ProviderLayout> Layouts { get; set; } = new();

    public int RefreshMinutes { get; set; } = 5;
    public string Language { get; set; } = "en";
    public int ChartHours { get; set; } = 24;
    public bool NotifyEnabled { get; set; }
    public HashSet<int> NotifyThresholds { get; set; } = new() { 75, 90, 95 };

    // ── 영속화 ──
    private static readonly JsonSerializerOptions JsonOpts = new()
    {
        WriteIndented = true,
        Converters = { new JsonStringEnumConverter() },
    };

    public static string DefaultPath =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
                     "UsageMeter", "settings.json");

    public static AppSettings Load(string? path = null)
    {
        path ??= DefaultPath;
        try
        {
            if (File.Exists(path))
                return JsonSerializer.Deserialize<AppSettings>(File.ReadAllText(path), JsonOpts) ?? new AppSettings();
        }
        catch { /* 손상 시 기본값 */ }
        var fresh = new AppSettings();
        fresh.EnsureDefaultLayouts();
        return fresh;
    }

    public void Save(string? path = null)
    {
        path ??= DefaultPath;
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllText(path, JsonSerializer.Serialize(this, JsonOpts));
    }

    /// <summary>기본 분배: 첫 AI=상단, 둘째=하단, 셋째=좌·본문.</summary>
    public void EnsureDefaultLayouts()
    {
        var segByIndex = new[] { SegPart.HTop, SegPart.HBottom, SegPart.LMain, SegPart.RMain };
        var anchorBySeg = new Dictionary<SegPart, BorderCorner>
        {
            [SegPart.HTop] = BorderCorner.TopLeft,
            [SegPart.HBottom] = BorderCorner.BottomLeft,
            [SegPart.LMain] = BorderCorner.BottomLeft,
            [SegPart.RMain] = BorderCorner.TopRight,
        };
        for (int i = 0; i < ProviderSpec.All.Count; i++)
        {
            var spec = ProviderSpec.All[i];
            if (Layouts.ContainsKey(spec.Id)) continue;
            var s = segByIndex[i % segByIndex.Length];
            Layouts[spec.Id] = new ProviderLayout
            {
                Segments = new HashSet<SegPart> { s },
                AnchorCorner = anchorBySeg[s],
            };
        }
    }

    public ProviderLayout LayoutFor(string id)
    {
        if (!Layouts.TryGetValue(id, out var l))
        {
            EnsureDefaultLayouts();
            l = Layouts.TryGetValue(id, out var l2) ? l2 : new ProviderLayout();
            Layouts[id] = l;
        }
        return l;
    }
}
