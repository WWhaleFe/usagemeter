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

    static let all: [ProviderSpec] = [claude, gemini]
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
          return {ok:true, five_hour: pick(u.five_hour), seven_day: pick(u.seven_day)};
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
}
