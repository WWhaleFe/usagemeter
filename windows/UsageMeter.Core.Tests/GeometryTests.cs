using UsageMeter.Core;
using Xunit;

namespace UsageMeter.Core.Tests;

/// <summary>macOS 디버그 하네스에서 하던 검증을 단위 테스트로 — 맥에서도 실행 가능.</summary>
public class SegGraphTests
{
    [Fact]
    public void FullLoop_IsValid()
    {
        var set = new HashSet<SegPart> { SegPart.HTop, SegPart.RMain, SegPart.HBottom, SegPart.LMain };
        Assert.True(SegGraph.IsValid(set, taskOn: false));
        Assert.Null(SegGraph.ChainEnds(set, taskOn: false));   // 고리 → 끝 없음
    }

    [Fact]
    public void OpenChain_HasTwoEnds()
    {
        var set = new HashSet<SegPart> { SegPart.HTop, SegPart.RMain };
        Assert.True(SegGraph.IsValid(set, taskOn: false));
        Assert.NotNull(SegGraph.ChainEnds(set, taskOn: false));
    }

    [Fact]
    public void TaskSegments_RequireTaskLine()
    {
        var set = new HashSet<SegPart> { SegPart.HTask };
        Assert.False(SegGraph.IsValid(set, taskOn: false));
        Assert.True(SegGraph.IsValid(set, taskOn: true));
    }

    [Fact]
    public void Disconnected_IsInvalid()
    {
        var set = new HashSet<SegPart> { SegPart.HTop, SegPart.HBottom };
        Assert.False(SegGraph.IsValid(set, taskOn: false));
    }

    [Fact]
    public void ZonePerimeter_TaskOnly_WrapsTaskBand()
    {
        var set = SegGraph.ZonePerimeter(new[] { ScreenZone.Task }, taskOn: true);
        Assert.Equal(new HashSet<SegPart> { SegPart.HTask, SegPart.HBottom, SegPart.LTask, SegPart.RTask }, set);
        Assert.True(SegGraph.IsValid(set, taskOn: true));
    }

    [Fact]
    public void ZonePerimeter_All_IsScreenLoop()
    {
        var set = SegGraph.ZonePerimeter(new[] { ScreenZone.Main, ScreenZone.Task }, taskOn: true);
        Assert.True(SegGraph.IsValid(set, taskOn: true));
        Assert.Null(SegGraph.ChainEnds(set, taskOn: true));
        Assert.Contains(SegPart.HTop, set);
        Assert.Contains(SegPart.HBottom, set);
    }
}

public class BorderGeometryTests
{
    private static BorderConfig LoopConfig(double thickness = 7, AnchorSide side = AnchorSide.Left) => new()
    {
        Segments = new HashSet<SegPart> { SegPart.HTop, SegPart.RMain, SegPart.HBottom, SegPart.LMain },
        TaskOn = false,
        Inset = thickness / 2,
        MainRadii = new()
        {
            [BorderCorner.TopLeft] = 22, [BorderCorner.TopRight] = 22,
            [BorderCorner.BottomLeft] = 22, [BorderCorner.BottomRight] = 22,
        },
        FullWidth = thickness,
        AnchorCorner = BorderCorner.TopLeft,
        AnchorSide = side,
    };

    [Fact]
    public void Loop_BuildsClosedPath()
    {
        var p = BorderGeometry.Build(LoopConfig(), 1920, 1080);
        Assert.NotNull(p);
        Assert.True(p!.IsLoop);
        Assert.True(p.Total > 2 * (1920 + 1080) * 0.8);   // 둘레에 근접
        // 닫힘: 시작점과 끝점이 일치(1pt 이내).
        var first = p.Samples[0];
        var last = p.Samples[^1];
        Assert.True(Math.Abs(first.X - last.X) < 1 && Math.Abs(first.Y - last.Y) < 1,
            $"시작 ({first.X:F1},{first.Y:F1}) vs 끝 ({last.X:F1},{last.Y:F1})");
    }

    [Fact]
    public void Spans_CoverWholePath_NoHoles()
    {
        var p = BorderGeometry.Build(LoopConfig(), 1920, 1080)!;
        Assert.True(p.Spans[0].From <= 0.0001);
        Assert.True(p.Spans[^1].To >= 0.9999);
        for (int i = 1; i < p.Spans.Count; i++)
            Assert.True(p.Spans[i].From <= p.Spans[i - 1].To + 0.0001,
                $"스팬 구멍: {p.Spans[i - 1].To} → {p.Spans[i].From}");
    }

    [Fact]
    public void AnchorSide_UpDown_StartDiffers()
    {
        var up = BorderGeometry.Build(LoopConfig(side: AnchorSide.Left), 1920, 1080)!;
        var down = BorderGeometry.Build(LoopConfig(side: AnchorSide.Right), 1920, 1080)!;
        var d = Math.Abs(up.Samples[0].X - down.Samples[0].X) + Math.Abs(up.Samples[0].Y - down.Samples[0].Y);
        Assert.True(d > 5, $"위/아래 시작점이 같음 (차이 {d:F2})");
    }

    [Fact]
    public void TaskBand_WithScoopCaps_Builds()
    {
        // 작업표시줄 감싸개: HTask(캡 위) + 좌우 + 하단 — macOS 'Dock 감싸개'의 Windows 대응.
        var cfg = new BorderConfig
        {
            Segments = new HashSet<SegPart> { SegPart.HTask, SegPart.LTask, SegPart.HBottom, SegPart.RTask },
            TaskOn = true,
            TaskH = 48,
            Inset = 3.5,
            MainRadii = new() { [BorderCorner.BottomLeft] = 20, [BorderCorner.BottomRight] = 20 },
            TaskRadii = new()
            {
                [BorderCorner.TopLeft] = 12, [BorderCorner.TopRight] = 12,
                [BorderCorner.BottomLeft] = 12, [BorderCorner.BottomRight] = 12,
            },
            HCaps = new()
            {
                [SegGraph.HCapKey(SegPart.HTask, right: false)] = SegCap.Up,
                [SegGraph.HCapKey(SegPart.HTask, right: true)] = SegCap.Up,
            },
            FullWidth = 7,
            AnchorCorner = BorderCorner.BottomLeft,
        };
        var p = BorderGeometry.Build(cfg, 1920, 1080);
        Assert.NotNull(p);
        Assert.True(p!.IsLoop);
        // 스쿱: 경로가 경계선(y=1032) 위로 넘어갔다 되돌아옴.
        double taskLineY = 1080 - 48;
        Assert.Contains(p.Samples, s => s.Y < taskLineY - 5);
    }

    [Fact]
    public void LaneSplit_ProducesNarrowSpans()
    {
        var cfg = new BorderConfig
        {
            Segments = new HashSet<SegPart> { SegPart.HTop, SegPart.RMain, SegPart.HBottom, SegPart.LMain },
            TaskOn = false,
            Inset = 3.5,
            MainRadii = new(),
            FullWidth = 7,
            LaneWidths = new() { [SegPart.HTop] = 3.5 },
            LaneOffsets = new() { [SegPart.HTop] = -1.75 },
        };
        var p = BorderGeometry.Build(cfg, 1920, 1080)!;
        Assert.Contains(p.Spans, s => Math.Abs(s.Width - 3.5) < 0.01);
        Assert.Contains(p.Spans, s => Math.Abs(s.Width - 7.0) < 0.01);
    }

    [Fact]
    public void Extract_ReturnsSubPolyline()
    {
        var p = BorderGeometry.Build(LoopConfig(), 1920, 1080)!;
        var half = p.Extract(0, 0.5);
        Assert.True(half.Count >= 2);
        var full = p.Extract(0, 1);
        Assert.True(full.Count >= half.Count);
        var none = p.Extract(0.5, 0.5);
        Assert.Empty(none);
    }

    [Fact]
    public void Smooth_LongTransitions_NoJumps()
    {
        var spans = new List<WidthSpan>
        {
            new(0, 0.3, 7), new(0.3, 0.31, 7), new(0.31, 0.7, 3.5), new(0.7, 1, 7),
        };
        var smooth = BorderGeometry.Smooth(spans, total: 3000, thickness: 7);
        // 인접 스팬 간 굵기 점프가 0.5pt 미만(평활 확인).
        for (int i = 1; i < smooth.Count; i++)
            Assert.True(Math.Abs(smooth[i].Width - smooth[i - 1].Width) < 0.5,
                $"급격한 굵기 점프: {smooth[i - 1].Width:F2} → {smooth[i].Width:F2}");
        // 중간이 실제로 얇아졌는지.
        Assert.Contains(smooth, s => s.Width < 5.5);
    }
}

public class ProviderTests
{
    [Fact]
    public void Snapshot_Parses_ClaudeShape()
    {
        var json = System.Text.Json.JsonDocument.Parse("""
            {"ok":true,
             "five_hour":{"utilization":25,"resets_at":"2026-07-06T03:00:00Z"},
             "seven_day":{"utilization":40,"resets_at":null},
             "seven_day_opus":{"utilization":10,"resets_at":null}}
            """);
        var s = UsageSnapshot.Parse("claude", json.RootElement, DateTimeOffset.UtcNow);
        Assert.True(s.Ok);
        Assert.Equal(0.75, s.RemainingRatio, 3);
        Assert.Equal(0.60, s.SecondaryRatio!.Value, 3);
        Assert.Equal(0.90, s.OpusRatio!.Value, 3);
        Assert.NotNull(s.ResetAt);
    }

    [Fact]
    public void Snapshot_NotLoggedIn_IsNotOk()
    {
        var json = System.Text.Json.JsonDocument.Parse("""{"ok":false,"reason":"not_logged_in"}""");
        var s = UsageSnapshot.Parse("claude", json.RootElement, DateTimeOffset.UtcNow);
        Assert.False(s.Ok);
    }

    [Fact]
    public void Settings_RoundTrip()
    {
        var tmp = Path.Combine(Path.GetTempPath(), $"um_test_{Guid.NewGuid():N}.json");
        try
        {
            var s = new AppSettings { Thickness = 5, TaskLineEnabled = true, TaskLineHeight = 52 };
            s.EnsureDefaultLayouts();
            s.LayoutFor("claude").Segments = new() { SegPart.HTask, SegPart.LTask };
            s.LayoutFor("claude").HCaps[SegGraph.HCapKey(SegPart.HTask, true)] = SegCap.Up;
            s.Save(tmp);
            var r = AppSettings.Load(tmp);
            Assert.Equal(5, r.Thickness);
            Assert.True(r.TaskLineEnabled);
            Assert.Equal(52, r.TaskLineHeight);
            Assert.Equal(s.LayoutFor("claude").Segments, r.LayoutFor("claude").Segments);
            Assert.Equal(SegCap.Up, r.LayoutFor("claude").HCaps[SegGraph.HCapKey(SegPart.HTask, true)]);
        }
        finally { File.Delete(tmp); }
    }
}
