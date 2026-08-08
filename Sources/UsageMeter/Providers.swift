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
    /// 로그인 창이 열 주소. nil이면 homeURL.
    /// 홈이 **로그아웃 상태에서 빈 화면을 주는** 서비스(제미나이)는 실제 로그인 폼이
    /// 서버에서 그려지는 주소를 따로 지정해야 한다. 안 그러면 흰 창만 뜨고 멈춘다.
    var loginURL: URL? = nil
    /// 로그인 완료 판정에 쓰는 인증 쿠키 이름들(하나라도 있으면 로그인으로 본다).
    /// 사용량 조회가 DOM 파싱이라 로그인 판정에 쓸 수 없는 서비스(제미나이)용.
    /// 비어 있으면 usageJS 결과(ok)로 판정한다.
    var authCookieNames: [String] = []
    /// 실제로 로그인 창이 열 주소.
    var effectiveLoginURL: URL { loginURL ?? homeURL }
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
          const org = orgs[0];
          // 구독 플랜(rate_limit_tier) → 사람이 읽는 라벨.
          const planLabel = ((t) => {
            if (!t) return null;
            if (t.includes('max_20x')) return 'Max (20x)';
            if (t.includes('max_5x')) return 'Max (5x)';
            if (t.includes('max')) return 'Max';
            if (t.includes('team')) return 'Team';
            if (t.includes('enterprise')) return 'Enterprise';
            if (t.includes('pro')) return 'Pro';
            if (t.includes('free')) return 'Free';
            return t.replace(/^default_claude_/,'').replace(/_/g,' ');
          })(org.rate_limit_tier || '');
          const usageRes = await fetch('https://claude.ai/api/organizations/'+org.uuid+'/usage', {headers:{'Accept':'application/json'}, credentials:'include'});
          if (usageRes.status !== 200) return {ok:false, reason:'usage_failed', status: usageRes.status};
          const u = await usageRes.json();
          const pick = (x) => (x && typeof x.utilization === 'number') ? {utilization: x.utilization, resets_at: x.resets_at || null} : null;
          // 모델별 주간 한도(weekly_scoped): 서버가 주는 display_name(예: "Fable") + percent(사용률).
          const buckets = Array.isArray(u.limits) ? u.limits
            .filter(l => l && l.kind === 'weekly_scoped' && l.scope && l.scope.model && l.scope.model.display_name && typeof l.percent === 'number')
            .map(l => ({label: l.scope.model.display_name, utilization: l.percent, resets_at: l.resets_at || null})) : [];
          return {ok:true, plan: planLabel, five_hour: pick(u.five_hour), seven_day: pick(u.seven_day), seven_day_opus: pick(u.seven_day_opus), model_buckets: buckets};
        } catch (e) { return {ok:false, reason:'exception', message: String(e)}; }
        """,
        defaultColor: Color(red: 0.85, green: 0.47, blue: 0.34)   // Anthropic 코랄
    )

    // MARK: - Gemini
    // gemini.google.com/usage 의 렌더된 DOM에서 5시간/주간 %를 읽는다(2026-05 도입).
    //
    // 로그인은 gemini.google.com이 아니라 구글 계정 페이지에서 해야 한다:
    // 로그아웃 상태의 gemini.google.com/usage는 서버 리다이렉트 없이 **본문이 빈 셸**만
    // 내려줘서(2026-08 확인) 로그인 창이 흰 화면으로 멈춘다. accounts.google.com의
    // ServiceLogin은 로그인 폼을 서버에서 그려주고, 끝나면 continue= 로 되돌아온다.
    static let gemini = ProviderSpec(
        id: "gemini",
        name: "Gemini",
        homeURL: URL(string: "https://gemini.google.com/usage")!,
        matchHost: "gemini.google.com",
        cookieDomains: ["google.com", "gemini"],
        loginURL: URL(string: "https://accounts.google.com/ServiceLogin?continue=https%3A%2F%2Fgemini.google.com%2Fusage&followup=https%3A%2F%2Fgemini.google.com%2Fusage")!,
        authCookieNames: ["__Secure-1PSID", "__Secure-3PSID", "SID"],
        reloadBeforeFetch: true,
        usageJS: """
        try {
          // 0) 실제 로그인 상태 확인: 구글 로그인 화면으로 튕겼으면 미로그인(재로그인 유도가 맞음).
          const host = location.host || '';
          const path = location.pathname || '';
          if (/accounts\\.google\\.com/.test(host) || /(^|\\/)(signin|ServiceLogin|InteractiveLogin)/i.test(path)) {
            return {ok:false, reason:'not_logged_in', host: host};
          }
          const sleep = (ms) => new Promise(r => setTimeout(r, ms));
          const pctFrom = (el) => {
            if (!el) return null;
            const m = (el.textContent || '').match(/(\\d+(?:\\.\\d+)?)\\s*%/);
            return m ? parseFloat(m[1]) : null;
          };
          const curSels = ['[data-test-id=\\"gxu-currently\\"]', '.gxu-currently', '[data-test-id*=\\"current\\"]'];
          const wkSels  = ['[data-test-id=\\"gxu-weekly\\"]', '.gxu-weekly', '[data-test-id*=\\"weekly\\"]'];
          const firstMatch = (sels) => { for (const s of sels) { const v = pctFrom(document.querySelector(s)); if (v != null) return v; } return null; };
          // 1) 클라이언트 렌더 대기: 셀렉터가 나타날 때까지 최대 ~6초 폴링.
          let cur = null, wk = null;
          for (let i = 0; i < 12; i++) {
            if (cur == null) cur = firstMatch(curSels);
            if (wk == null)  wk  = firstMatch(wkSels);
            if (cur != null && wk != null) break;
            await sleep(500);
          }
          // 2) 셀렉터 실패 시: 사용/남은 관련 단어 근처의 %만 채택(임의의 % 오탐 방지).
          //    강함(같은 잎에 키워드+%) 우선, 없으면 약함(부모 문맥에 키워드)으로 폴백.
          if (cur == null && wk == null) {
            const KW = /(usage|used|limit|remaining|left|quota|사용|남은|한도|使用|残)/i;
            const strong = [], weak = [];
            const leaves = document.querySelectorAll('body *');
            for (const n of leaves) {
              if (n.children && n.children.length > 0) continue;      // 잎 노드만
              const txt = (n.textContent || '').trim();
              const m = txt.match(/(\\d+(?:\\.\\d+)?)\\s*%/);
              if (!m) continue;
              const val = parseFloat(m[1]);
              if (KW.test(txt)) { strong.push(val); }
              else { const ctx = (n.parentElement && n.parentElement.textContent) || ''; if (KW.test(ctx)) weak.push(val); }
            }
            const cands = strong.length ? strong : weak;
            if (cands.length) { cur = cands[0]; wk = cands.length > 1 ? cands[1] : null; }
          }
          // 3) 그래도 못 찾으면 두 경우를 갈라야 한다.
          //    (a) 로그아웃: /usage가 서버 리다이렉트 없이 빈 셸을 주므로 호스트로는 못 가린다.
          //        대신 구글바의 'Sign in'(ServiceLogin) 링크가 로그아웃 상태에서만 나온다.
          //        %를 읽는 데 성공했으면 여기까지 오지 않으므로 오탐 위험이 없다.
          //    (b) 로그인은 됐는데 화면을 못 읽음 → '읽기 실패'(재로그인 요구 아님).
          if (cur == null && wk == null) {
            if (document.querySelector('a[href*="ServiceLogin"], a[href*="accounts.google.com/signin"]')) {
              return {ok:false, reason:'not_logged_in', host: host};
            }
            const snippet = (document.body ? (document.body.innerText || '') : '').slice(0, 200).replace(/\\s+/g, ' ').trim();
            return {ok:false, reason:'parse_failed', host: host, snippet: snippet};
          }
          const pick = (p) => (p == null) ? null : {utilization: p, resets_at: null};
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
          // 2) 계정 ID (팀 워크스페이스 구분용 헤더, 실패해도 진행) + 구독 플랜(best-effort).
          let accountId = null;
          let plan = null;
          if (token) {
            try {
              const accRes = await fetch('https://chatgpt.com/backend-api/accounts/check/v4-2023-04-27', {headers:{'Authorization':'Bearer '+token,'Accept':'application/json'}, credentials:'include'});
              if (accRes.status === 200) {
                const acc = await accRes.json();
                const entries = (acc && acc.accounts) ? Object.values(acc.accounts) : [];
                const pick = entries.find(a => a && a.account && a.account.is_default) || entries[0];
                accountId = (pick && pick.account && (pick.account.account_id || pick.account.id)) || null;
                const pt = pick && pick.account && (pick.account.plan_type || pick.account.plan || (pick.account.entitlement && pick.account.entitlement.subscription_plan));
                if (typeof pt === 'string' && pt) {
                  const m = {plus:'Plus', pro:'Pro', team:'Team', free:'Free', enterprise:'Enterprise', business:'Business'};
                  const key = pt.replace(/^chatgpt[-_]?/,'').replace(/[-_].*$/,'').toLowerCase();
                  plan = m[key] || pt;
                }
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
          return {ok:true, plan: plan, five_hour: primary, seven_day: secondary};
        } catch (e) { return {ok:false, reason:'exception', message: String(e)}; }
        """,
        defaultColor: Color(red: 0.06, green: 0.64, blue: 0.50)   // OpenAI 틸
    )
}
