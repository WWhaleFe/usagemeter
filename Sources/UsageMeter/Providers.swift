import SwiftUI

/// 하나의 AI 서비스(클로드/제미나이 등) 명세. WebSession이 이걸 받아
/// 로그인·조회를 그 서비스에 맞게 수행한다. 지금은 Claude·Gemini만 지원.
struct ProviderSpec: Identifiable {
    let id: String
    let name: String
    let homeURL: URL
    /// 이 서비스 페이지인지 식별할 호스트 조각(예: "claude.ai").
    let matchHost: String
    /// 로그아웃 시 지울 쿠키 도메인 조각들.
    let cookieDomains: [String]
    /// 조회 전 홈을 다시 로드해야 하는지(제미나이 /usage는 재로드로 최신 DOM 확보).
    let reloadBeforeFetch: Bool
    /// 사용량을 읽는 async JS. 반환: {ok, five_hour:{utilization,resets_at}, seven_day:{...}} 또는 {ok:false,...}
    let usageJS: String
    /// 기본 색(브랜드 색).
    let defaultColor: Color

    static let all: [ProviderSpec] = [claude, gemini, codex]
    static func spec(_ id: String) -> ProviderSpec? { all.first { $0.id == id } }

    // MARK: - Claude
    static let claude = ProviderSpec(
        id: "claude",
        name: "Claude",
        homeURL: URL(string: "https://claude.ai/")!,
        matchHost: "claude.ai",
        cookieDomains: ["claude", "anthropic"],
        reloadBeforeFetch: false,
        usageJS: """
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
        """,
        defaultColor: Color(red: 0.85, green: 0.47, blue: 0.34)   // Anthropic 코랄
    )

    // MARK: - Gemini
    // gemini.google.com/usage 의 렌더된 DOM에서 5시간/주간 %를 읽는다(2026-05 도입).
    static let gemini = ProviderSpec(
        id: "gemini",
        name: "Gemini",
        homeURL: URL(string: "https://gemini.google.com/usage")!,
        matchHost: "gemini.google.com",
        cookieDomains: ["google.com", "gemini"],
        reloadBeforeFetch: true,
        usageJS: """
        try {
          function pct(el){ if(!el) return null; const m=(el.textContent||'').match(/(\\d+)\\s*%/); return m?parseInt(m[1],10):null; }
          let cur = pct(document.querySelector('[data-test-id=\\"gxu-currently\\"]')) ?? pct(document.querySelector('.gxu-currently'));
          let wk  = pct(document.querySelector('[data-test-id=\\"gxu-weekly\\"]')) ?? pct(document.querySelector('.gxu-weekly'));
          if (cur==null && wk==null) {
            const t = document.body ? (document.body.innerText||'') : '';
            const ms = [...t.matchAll(/(\\d+)\\s*%/g)].map(m=>parseInt(m[1],10));
            if (ms.length) { cur = ms[0]; wk = ms.length>1 ? ms[1] : null; }
          }
          if (cur==null && wk==null) return {ok:false, reason:'no_data'};
          const pick = (p) => (p==null) ? null : {utilization: p, resets_at: null};
          return {ok:true, five_hour: pick(cur), seven_day: pick(wk)};
        } catch (e) { return {ok:false, reason:'exception', message: String(e)}; }
        """,
        defaultColor: Color(red: 0.26, green: 0.52, blue: 0.96)   // Google 블루
    )

    // MARK: - Codex (ChatGPT 플랜)
    // chatgpt.com 로그인 세션으로 Codex 공식 대시보드(chatgpt.com/codex/settings/usage)가 쓰는
    // 내부 엔드포인트 `backend-api/wham/usage`를 호출한다. 5시간 창(primary) + 주간(secondary).
    // 이 한도는 계정 전체(모든 디바이스의 Codex CLI·IDE·클라우드 사용 합산) 기준이다.
    // 비공식 엔드포인트라 필드명 변형(used_percent/usage_percent, reset_at/reset_after_seconds 등)을
    // 모두 수용하도록 방어적으로 파싱한다. DOM 셀렉터는 쓰지 않는다(UI 개편에 안 깨지게).
    static let codex = ProviderSpec(
        id: "codex",
        name: "Codex",
        homeURL: URL(string: "https://chatgpt.com/")!,
        matchHost: "chatgpt.com",
        cookieDomains: ["chatgpt", "openai"],
        reloadBeforeFetch: false,
        usageJS: """
        try {
          // 1) 액세스 토큰: 로그인된 세션에서 발급받는다 (미로그인 시 accessToken 없음).
          let token = null;
          try {
            const sessRes = await fetch('https://chatgpt.com/api/auth/session', {headers:{'Accept':'application/json'}, credentials:'include'});
            if (sessRes.status === 401 || sessRes.status === 403) return {ok:false, reason:'not_logged_in', status: sessRes.status};
            if (sessRes.status === 200) { const s = await sessRes.json(); token = (s && s.accessToken) || null; }
          } catch (e) {}
          // 2) 계정 ID (팀 워크스페이스 구분용 헤더, 실패해도 진행).
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
          // 3) 사용량 조회 (Codex 대시보드와 동일한 내부 엔드포인트). 토큰 없으면 쿠키만으로 시도.
          const headers = {'Accept':'application/json'};
          if (token) headers['Authorization'] = 'Bearer ' + token;
          if (accountId) headers['ChatGPT-Account-ID'] = accountId;
          const uRes = await fetch('https://chatgpt.com/backend-api/wham/usage', {headers, credentials:'include'});
          if (uRes.status === 401 || uRes.status === 403) return {ok:false, reason:'not_logged_in', status: uRes.status};
          if (uRes.status !== 200) return {ok:false, reason:'usage_failed', status: uRes.status};
          const u = await uRes.json();
          const rl = (u && (u.rate_limit || u.rate_limits)) || null;
          if (!rl) return {ok:false, reason:'no_data'};
          // 필드명 변형을 흡수하는 헬퍼들.
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
        """,
        defaultColor: Color(red: 0.06, green: 0.64, blue: 0.50)   // OpenAI 틸
    )
}
