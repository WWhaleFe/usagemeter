namespace UsageMeter.Core;

/// <summary>
/// 화면 골격의 선분(세그먼트) — Windows 2분할 모델.
/// 가로 3개(상단 / 작업표시줄선 / 하단) + 세로 좌·우 각 2구간(본문 / 작업표시줄).
/// macOS의 메뉴바 경계선은 Windows에 없으므로 제거하고, Dock선은 '작업표시줄 경계선'으로 대응.
/// 작업표시줄 경계선이 꺼져 있으면 관련 세그먼트는 선택 불가(본문이 화면 전체).
/// </summary>
public enum SegPart
{
    HTop, HTask, HBottom,        // 가로선 3개
    LMain, LTask,                // 좌변 2구간
    RMain, RTask,                // 우변 2구간
}

public static class SegPartExtensions
{
    public static bool IsHorizontal(this SegPart s) =>
        s is SegPart.HTop or SegPart.HTask or SegPart.HBottom;
}

/// <summary>세그먼트 그래프의 노드: (좌/우) × (가로 레벨 0=상단 1=작업표시줄선 2=하단).</summary>
public readonly record struct SegNode(bool Right, int Level) : IComparable<SegNode>
{
    public int CompareTo(SegNode other)
    {
        var c = Level.CompareTo(other.Level);
        return c != 0 ? c : (Right ? 1 : 0).CompareTo(other.Right ? 1 : 0);
    }
}

/// <summary>열린 체인 끝의 모서리 캡 방향.</summary>
public enum SegCap { None, Up, Down, Include }

/// <summary>차감 시작 지점(모서리 곡선 기준): 위 / 모서리(스쿱) / 아래.</summary>
public enum AnchorSide { Left, Center, Right }

/// <summary>시작 꼭짓점.</summary>
public enum BorderCorner { TopLeft, TopRight, BottomRight, BottomLeft }

/// <summary>화면 영역(2분할): 본문 / 작업표시줄 띠.</summary>
public enum ScreenZone { Main, Task }

/// <summary>세그먼트 그래프 헬퍼 — macOS OverlaySettings의 static 함수군을 2분할로 이식.</summary>
public static class SegGraph
{
    /// <summary>이 세그먼트가 현재 경계선 설정에서 존재하는지.</summary>
    public static bool Available(SegPart s, bool taskOn) => s switch
    {
        SegPart.HTask or SegPart.LTask or SegPart.RTask => taskOn,
        _ => true,
    };

    /// <summary>세그먼트의 양 끝 노드(본문 세로선은 경계선 유무에 따라 늘어난다).</summary>
    public static (SegNode A, SegNode B) Ends(SegPart s, bool taskOn)
    {
        int mainBot = taskOn ? 1 : 2;
        return s switch
        {
            SegPart.HTop => (new(false, 0), new(true, 0)),
            SegPart.HTask => (new(false, 1), new(true, 1)),
            SegPart.HBottom => (new(false, 2), new(true, 2)),
            SegPart.LMain => (new(false, 0), new(false, mainBot)),
            SegPart.LTask => (new(false, 1), new(false, 2)),
            SegPart.RMain => (new(true, 0), new(true, mainBot)),
            _ => (new(true, 1), new(true, 2)),   // RTask
        };
    }

    /// <summary>노드 → 인접 세그먼트 목록.</summary>
    public static Dictionary<SegNode, List<SegPart>> Adjacency(IEnumerable<SegPart> set, bool taskOn)
    {
        var adj = new Dictionary<SegNode, List<SegPart>>();
        foreach (var s in set)
        {
            var (a, b) = Ends(s, taskOn);
            (adj.TryGetValue(a, out var la) ? la : adj[a] = new()).Add(s);
            (adj.TryGetValue(b, out var lb) ? lb : adj[b] = new()).Add(s);
        }
        return adj;
    }

    /// <summary>선택이 '한 줄로 이어진 선(체인 또는 고리)'인지.</summary>
    public static bool IsValid(IReadOnlyCollection<SegPart> set, bool taskOn)
    {
        if (set.Count == 0) return false;
        if (!set.All(s => Available(s, taskOn))) return false;
        var adj = Adjacency(set, taskOn);
        if (adj.Values.Any(l => l.Count > 2)) return false;
        int ends = adj.Values.Count(l => l.Count == 1);
        if (ends != 0 && ends != 2) return false;
        // 연결성: BFS로 전부 도달해야 함.
        var visited = new HashSet<SegPart>();
        var queue = new Queue<SegPart>();
        queue.Enqueue(set.First());
        while (queue.Count > 0)
        {
            var s = queue.Dequeue();
            if (!visited.Add(s)) continue;
            var (a, b) = Ends(s, taskOn);
            foreach (var n in new[] { a, b })
                if (adj.TryGetValue(n, out var list))
                    foreach (var nx in list)
                        if (!visited.Contains(nx)) queue.Enqueue(nx);
        }
        return visited.Count == set.Count;
    }

    /// <summary>열린 체인의 두 끝 (노드, 세그먼트). 고리/무효면 null.</summary>
    public static ((SegNode Node, SegPart Seg) Start, (SegNode Node, SegPart Seg) End)?
        ChainEnds(IReadOnlyCollection<SegPart> set, bool taskOn)
    {
        if (!IsValid(set, taskOn)) return null;
        var adj = Adjacency(set, taskOn);
        var eps = adj.Where(kv => kv.Value.Count == 1).Select(kv => kv.Key).OrderBy(n => n).ToList();
        if (eps.Count != 2) return null;
        return ((eps[0], adj[eps[0]][0]), (eps[1], adj[eps[1]][0]));
    }

    /// <summary>영역 조합의 둘레 세그먼트(모양 프리셋용): 전체 / 본문 / 작업표시줄.</summary>
    public static HashSet<SegPart> ZonePerimeter(IReadOnlyCollection<ScreenZone> zones, bool taskOn)
    {
        var eff = zones.Select(z => z == ScreenZone.Task && !taskOn ? ScreenZone.Main : z).ToHashSet();
        if (eff.Count == 0) eff.Add(ScreenZone.Main);

        int TopLevel(ScreenZone z) => z == ScreenZone.Main ? 0 : 1;
        int BottomLevel(ScreenZone z) => z == ScreenZone.Main ? (taskOn ? 1 : 2) : 2;
        SegPart HSeg(int level) => level switch { 0 => SegPart.HTop, 1 => SegPart.HTask, _ => SegPart.HBottom };

        var top = eff.Contains(ScreenZone.Main) ? ScreenZone.Main : ScreenZone.Task;
        var bottom = eff.Contains(ScreenZone.Task) ? ScreenZone.Task : ScreenZone.Main;
        var outSet = new HashSet<SegPart> { HSeg(TopLevel(top)), HSeg(BottomLevel(bottom)) };
        if (eff.Contains(ScreenZone.Main)) { outSet.Add(SegPart.LMain); outSet.Add(SegPart.RMain); }
        if (eff.Contains(ScreenZone.Task) && taskOn) { outSet.Add(SegPart.LTask); outSet.Add(SegPart.RTask); }
        return outSet;
    }

    /// <summary>가로선 캡 딕셔너리 키: "HTask.L" / "HTask.R".</summary>
    public static string HCapKey(SegPart s, bool right) => $"{s}.{(right ? "R" : "L")}";
}
