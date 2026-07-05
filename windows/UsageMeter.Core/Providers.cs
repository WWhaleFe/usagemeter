namespace UsageMeter.Core;

/// <summary>
/// 한 AI 서비스의 조회 명세 — macOS ProviderSpec 이식.
/// UsageJs는 WebView2의 ExecuteScriptAsync로 실행되는 async IIFE 본문이며,
/// 결과는 {ok, five_hour:{utilization,resets_at}, seven_day:{...}, seven_day_opus:{...}} JSON.
/// </summary>
public sealed record ProviderSpec(
    string Id,
    string Name,
    string HomeUrl,
    string MatchHost,
    bool ReloadBeforeFetch,
    string UsageJs,
    (byte R, byte G, byte B) DefaultColor)
{
    public static readonly ProviderSpec Claude = new(
        Id: "claude",
        Name: "Claude",
        HomeUrl: "https://claude.ai/",
        MatchHost: "claude.ai",
        ReloadBeforeFetch: false,
        UsageJs: """
        (async () => {
          try {
            const orgsRes = await fetch('https://claude.ai/api/organizations', {headers:{'Accept':'application/json'}, credentials:'include'});
            if (orgsRes.status !== 200) return {ok:false, reason:'not_logged_in', status: orgsRes.status};
            const orgs = await orgsRes.json();
            if (!Array.isArray(orgs) || orgs.length === 0) return {ok:false, reason:'no_org'};
            const org = orgs[0].uuid;
            const usageRes = await fetch('https://claude.ai/api/organizations/'+org+'/usage', {headers:{'Accept':'application/json'}, credentials:'include'});
            if (usageRes.status !== 200) return {ok:false, reason:'usage_failed', status: usageRes.status};
            const u = await usageRes.json();
            const pick = (x) => (x && typeof x.utilization === 'number') ? {utilization: x.utilization, resets_at: x.resets_at || null} : null;
            return {ok:true, five_hour: pick(u.five_hour), seven_day: pick(u.seven_day), seven_day_opus: pick(u.seven_day_opus)};
          } catch (e) { return {ok:false, reason:'exception', message: String(e)}; }
        })()
        """,
        DefaultColor: (217, 120, 87));            // Anthropic 코랄 #D97757

    public static readonly ProviderSpec Gemini = new(
        Id: "gemini",
        Name: "Gemini",
        HomeUrl: "https://gemini.google.com/usage",
        MatchHost: "gemini.google.com",
        ReloadBeforeFetch: true,
        UsageJs: """
        (async () => {
          try {
            function pct(el){ if(!el) return null; const m=(el.textContent||'').match(/(\d+)\s*%/); return m?parseInt(m[1],10):null; }
            let cur = pct(document.querySelector('[data-test-id="gxu-currently"]')) ?? pct(document.querySelector('.gxu-currently'));
            let wk  = pct(document.querySelector('[data-test-id="gxu-weekly"]')) ?? pct(document.querySelector('.gxu-weekly'));
            if (cur==null && wk==null) {
              const t = document.body ? (document.body.innerText||'') : '';
              const ms = [...t.matchAll(/(\d+)\s*%/g)].map(m=>parseInt(m[1],10));
              if (ms.length) { cur = ms[0]; wk = ms.length>1 ? ms[1] : null; }
            }
            if (cur==null && wk==null) return {ok:false, reason:'no_data'};
            const pick = (p) => (p==null) ? null : {utilization: p, resets_at: null};
            return {ok:true, five_hour: pick(cur), seven_day: pick(wk)};
          } catch (e) { return {ok:false, reason:'exception', message: String(e)}; }
        })()
        """,
        DefaultColor: (66, 133, 244));            // Google 블루 #4285F4

    public static readonly ProviderSpec Codex = new(
        Id: "codex",
        Name: "Codex",
        HomeUrl: "https://chatgpt.com/",
        MatchHost: "chatgpt.com",
        ReloadBeforeFetch: false,
        UsageJs: """
        (async () => {
          try {
            let token = null;
            try {
              const sessRes = await fetch('https://chatgpt.com/api/auth/session', {headers:{'Accept':'application/json'}, credentials:'include'});
              if (sessRes.status === 401 || sessRes.status === 403) return {ok:false, reason:'not_logged_in', status: sessRes.status};
              if (sessRes.status === 200) { const s = await sessRes.json(); token = (s && s.accessToken) || null; }
            } catch (e) {}
            let accountId = null;
            if (token) {
              try {
                const accRes = await fetch('https://chatgpt.com/backend-api/accounts/check/v4-2023-04-27', {headers:{'Authorization':'Bearer '+token,'Accept':'application/json'}, credentials:'include'});
                if (accRes.status === 200) {
                  const acc = await accRes.json();
                  const entries = (acc && acc.accounts) ? Object.values(acc.accounts) : [];
                  const pick = entries.find(a => a && a.account && a.account.is_default) || entries[0];
                  accountId = (pick && pick.account && (pick.account.account_id || pick.account.id)) || null;
                }
              } catch (e) {}
            }
            const headers = {'Accept':'application/json'};
            if (token) headers['Authorization'] = 'Bearer ' + token;
            if (accountId) headers['ChatGPT-Account-ID'] = accountId;
            const uRes = await fetch('https://chatgpt.com/backend-api/wham/usage', {headers, credentials:'include'});
            if (uRes.status === 401 || uRes.status === 403) return {ok:false, reason:'not_logged_in', status: uRes.status};
            if (uRes.status !== 200) return {ok:false, reason:'usage_failed', status: uRes.status};
            const u = await uRes.json();
            const rl = (u && (u.rate_limit || u.rate_limits)) || null;
            if (!rl) return {ok:false, reason:'no_data'};
            const num = (v) => (typeof v === 'number' && isFinite(v)) ? v : null;
            const iso = (w) => {
              const after = num(w.reset_after_seconds) ?? num(w.resets_in_seconds);
              if (after != null) return new Date(Date.now() + after*1000).toISOString();
              const at = (w.reset_at != null) ? w.reset_at : w.resets_at;
              if (typeof at === 'number' && isFinite(at)) return new Date(at > 1e12 ? at : at*1000).toISOString();
              if (typeof at === 'string' && at) return at;
              return null;
            };
            const win = (w) => {
              if (!w) return null;
              const used = num(w.used_percent) ?? num(w.usage_percent) ?? num(w.utilization);
              if (used == null) return null;
              return {utilization: used, resets_at: iso(w)};
            };
            const primary = win(rl.primary_window || rl.primary);
            const secondary = win(rl.secondary_window || rl.secondary);
            if (!primary && !secondary) return {ok:false, reason:'no_data'};
            return {ok:true, five_hour: primary, seven_day: secondary};
          } catch (e) { return {ok:false, reason:'exception', message: String(e)}; }
        })()
        """,
        DefaultColor: (16, 163, 127));            // OpenAI 틸 #10A37F

    public static readonly IReadOnlyList<ProviderSpec> All = new[] { Claude, Gemini, Codex };
}

/// <summary>한 AI의 잔여 사용량 스냅샷 — macOS UsageSnapshot 이식.</summary>
public sealed record UsageSnapshot(
    string Id,
    double RemainingRatio,          // 0~1 (5시간 창)
    double? SecondaryRatio,         // 주간
    double? OpusRatio,              // Opus 주간(Claude)
    DateTimeOffset? ResetAt,
    DateTimeOffset? SecondaryResetAt,
    DateTimeOffset? OpusResetAt,
    bool Ok,
    DateTimeOffset LastUpdated)
{
    /// <summary>usage JS 결과 JSON을 스냅샷으로 파싱. 잔여 = 1 − utilization/100.</summary>
    public static UsageSnapshot Parse(string id, System.Text.Json.JsonElement root, DateTimeOffset now)
    {
        static (double Remaining, DateTimeOffset? Reset)? Pick(System.Text.Json.JsonElement parent, string key)
        {
            if (!parent.TryGetProperty(key, out var el) || el.ValueKind != System.Text.Json.JsonValueKind.Object)
                return null;
            if (!el.TryGetProperty("utilization", out var u) || u.ValueKind != System.Text.Json.JsonValueKind.Number)
                return null;
            double remaining = Math.Clamp(1.0 - u.GetDouble() / 100.0, 0, 1);
            DateTimeOffset? reset = null;
            if (el.TryGetProperty("resets_at", out var r) && r.ValueKind == System.Text.Json.JsonValueKind.String
                && DateTimeOffset.TryParse(r.GetString(), out var dt))
                reset = dt;
            return (remaining, reset);
        }
        bool ok = root.TryGetProperty("ok", out var okEl) && okEl.ValueKind == System.Text.Json.JsonValueKind.True;
        if (!ok)
            return new UsageSnapshot(id, 0, null, null, null, null, null, false, now);
        var five = Pick(root, "five_hour");
        var week = Pick(root, "seven_day");
        var opus = Pick(root, "seven_day_opus");
        return new UsageSnapshot(
            id,
            five?.Remaining ?? 1.0,
            week?.Remaining,
            opus?.Remaining,
            five?.Reset, week?.Reset, opus?.Reset,
            true, now);
    }
}
