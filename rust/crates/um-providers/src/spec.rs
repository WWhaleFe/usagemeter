//! AI 서비스 명세. (Swift Providers.swift `ProviderSpec`)
//!
//! `usage_js`는 Swift 멀티라인 리터럴이 WebKit에 넘기는 JS 본문과 바이트 단위로 같다
//! (들여쓰기 8칸 제거, `\\` -> `\`, `\"` -> `"` 적용 결과). 웹뷰 경로에서 그대로 재사용한다.

/// 하나의 AI 서비스 명세. (Swift `ProviderSpec`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProviderSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub home_url: &'static str,
    /// 이 서비스 페이지인지 식별할 호스트 조각.
    pub match_host: &'static str,
    /// 로그아웃 시 지울 쿠키 도메인 조각들.
    pub cookie_domains: &'static [&'static str],
    /// 로그인 창이 열 주소. None이면 `home_url`. (Gemini는 로그아웃 홈이 빈 화면이라 따로 지정)
    pub login_url: Option<&'static str>,
    /// 로그인 완료 판정용 인증 쿠키 이름들(하나라도 있으면 로그인). 비면 usage_js 결과(ok)로 판정.
    pub auth_cookie_names: &'static [&'static str],
    /// 조회 전 홈을 다시 로드해야 하는지.
    pub reload_before_fetch: bool,
    /// 렌더된 DOM이 필요한지(Gemini). false면 빈 오리진 문서로 충분.
    pub needs_rendered_dom: bool,
    /// 사용량을 읽는 async JS 본문(웹뷰 `callAsyncJavaScript`용).
    pub usage_js: &'static str,
    /// 기본 색(브랜드 색) RGB, 0.0~1.0.
    pub default_color: (f64, f64, f64),
}

impl ProviderSpec {
    /// 실제로 로그인 창이 열 주소. (Swift `effectiveLoginURL`)
    pub fn effective_login_url(&self) -> &'static str {
        self.login_url.unwrap_or(self.home_url)
    }
}

/// 표시 순서(스펙 순서). (Swift `ProviderSpec.all`)
pub static ALL: [&ProviderSpec; 3] = [&CLAUDE, &GEMINI, &CODEX];

/// id로 명세를 찾는다. (Swift `ProviderSpec.spec(_:)`)
pub fn spec(id: &str) -> Option<&'static ProviderSpec> {
    ALL.iter().copied().find(|s| s.id == id)
}

/// Anthropic 코랄.
pub static CLAUDE: ProviderSpec = ProviderSpec {
    id: "claude",
    name: "Claude",
    home_url: "https://claude.ai/",
    match_host: "claude.ai",
    cookie_domains: &["claude", "anthropic"],
    login_url: None,
    auth_cookie_names: &[],
    reload_before_fetch: false,
    needs_rendered_dom: false,
    usage_js: CLAUDE_USAGE_JS,
    default_color: (0.85, 0.47, 0.34),
};

/// Google 블루. 로그인은 gemini.google.com이 아니라 구글 계정 페이지에서 한다(Providers.swift:96-101).
pub static GEMINI: ProviderSpec = ProviderSpec {
    id: "gemini",
    name: "Gemini",
    home_url: "https://gemini.google.com/usage",
    match_host: "gemini.google.com",
    cookie_domains: &["google.com", "gemini"],
    login_url: Some(
        "https://accounts.google.com/ServiceLogin?continue=https%3A%2F%2Fgemini.google.com%2Fusage&followup=https%3A%2F%2Fgemini.google.com%2Fusage",
    ),
    auth_cookie_names: &["__Secure-1PSID", "__Secure-3PSID", "SID"],
    reload_before_fetch: true,
    needs_rendered_dom: true,
    usage_js: GEMINI_USAGE_JS,
    default_color: (0.26, 0.52, 0.96),
};

/// OpenAI 틸.
pub static CODEX: ProviderSpec = ProviderSpec {
    id: "codex",
    name: "Codex",
    home_url: "https://chatgpt.com/",
    match_host: "chatgpt.com",
    cookie_domains: &["chatgpt", "openai"],
    login_url: None,
    auth_cookie_names: &[],
    reload_before_fetch: false,
    needs_rendered_dom: false,
    usage_js: CODEX_USAGE_JS,
    default_color: (0.06, 0.64, 0.50),
};

const CLAUDE_USAGE_JS: &str = r####"try {
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
  return {ok:true, plan: planLabel, five_hour: pick(u.five_hour), seven_day: pick(u.seven_day), seven_day_opus: pick(u.seven_day_opus), seven_day_sonnet: pick(u.seven_day_sonnet), model_buckets: buckets};
} catch (e) { return {ok:false, reason:'exception', message: String(e)}; }"####;

const GEMINI_USAGE_JS: &str = r####"try {
  // 0) 실제 로그인 상태 확인: 구글 로그인 화면으로 튕겼으면 미로그인(재로그인 유도가 맞음).
  const host = location.host || '';
  const path = location.pathname || '';
  if (/accounts\.google\.com/.test(host) || /(^|\/)(signin|ServiceLogin|InteractiveLogin)/i.test(path)) {
    return {ok:false, reason:'not_logged_in', host: host};
  }
  const sleep = (ms) => new Promise(r => setTimeout(r, ms));
  const pctFrom = (el) => {
    if (!el) return null;
    const m = (el.textContent || '').match(/(\d+(?:\.\d+)?)\s*%/);
    return m ? parseFloat(m[1]) : null;
  };
  // 2026-11 개편: <li data-testid="current-usage"> / <li data-testid="weekly-usage">
  // (속성명이 data-test-id → data-testid로 바뀜). 옛 셀렉터는 호환용으로 뒤에 둔다.
  const curSels = ['[data-testid="current-usage"]', '[data-test-id=\"gxu-currently\"]', '.gxu-currently', '[data-test-id*=\"current\"]'];
  const wkSels  = ['[data-testid="weekly-usage"]', '[data-test-id=\"gxu-weekly\"]', '.gxu-weekly', '[data-test-id*=\"weekly\"]'];
  const firstMatch = (sels) => { for (const s of sels) { const el = document.querySelector(s); const v = pctFrom(el); if (v != null) return {v: v, el: el}; } return null; };
  // 리셋 문구 → ISO 시각. 화면은 절대 시각만 준다(로컬 시간대):
  //   ko "오후 11:18에 초기화" / "11월 3일 오후 5:18에 초기화"
  //   en "Resets at 11:18 PM" / "Resets Nov 3 at 5:18 PM"
  // 날짜가 없으면 오늘(이미 지났으면 내일), 날짜가 있으면 올해(이미 지났으면 내년).
  const RESET_KW = /(초기화|재설정|reset|リセット|重置|重設)/i;
  const MONTHS = {jan:1,feb:2,mar:3,apr:4,may:5,jun:6,jul:7,aug:8,sep:9,oct:10,nov:11,dec:12};
  const parseReset = (txt) => {
    const t = txt.match(/(\d{1,2})[:：](\d{2})/);
    if (!t) return null;
    let h = parseInt(t[1], 10); const mi = parseInt(t[2], 10);
    if (/(오후|PM|p\.m\.|午後|下午)/i.test(txt) && h < 12) h += 12;
    else if (/(오전|AM|a\.m\.|午前|上午)/i.test(txt) && h === 12) h = 0;
    let mo = null, d = null, m;
    if ((m = txt.match(/(\d{1,2})\s*[월月]\s*(\d{1,2})\s*[일日]/))) { mo = +m[1]; d = +m[2]; }
    else if ((m = txt.match(/\b(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\.?\s+(\d{1,2})\b/i))) { mo = MONTHS[m[1].toLowerCase()]; d = +m[2]; }
    else if ((m = txt.match(/\b(\d{1,2})\s+(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*/i))) { mo = MONTHS[m[2].toLowerCase()]; d = +m[1]; }
    const now = new Date();
    let at;
    if (mo != null) {
      at = new Date(now.getFullYear(), mo - 1, d, h, mi);
      if (at.getTime() < now.getTime() - 86400000) at.setFullYear(at.getFullYear() + 1);
    } else {
      at = new Date(now.getFullYear(), now.getMonth(), now.getDate(), h, mi);
      if (/(내일|tomorrow|明日)/i.test(txt) || at.getTime() < now.getTime() - 60000) at.setDate(at.getDate() + 1);
    }
    return isNaN(at.getTime()) ? null : at.toISOString();
  };
  const resetIn = (el) => {
    if (!el) return null;
    for (const n of el.querySelectorAll('*')) {
      if (n.children.length > 0) continue;
      const txt = (n.textContent || '').trim();
      if (RESET_KW.test(txt)) { const iso = parseReset(txt); if (iso) return iso; }
    }
    return null;
  };
  // 1) 클라이언트 렌더 대기: 셀렉터가 나타날 때까지 최대 ~6초 폴링.
  let cur = null, wk = null, curReset = null, wkReset = null;
  for (let i = 0; i < 12; i++) {
    if (cur == null) { const r = firstMatch(curSels); if (r) { cur = r.v; curReset = resetIn(r.el); } }
    if (wk == null)  { const r = firstMatch(wkSels);  if (r) { wk = r.v;  wkReset = resetIn(r.el); } }
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
      const m = txt.match(/(\d+(?:\.\d+)?)\s*%/);
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
    const snippet = (document.body ? (document.body.innerText || '') : '').slice(0, 200).replace(/\s+/g, ' ').trim();
    return {ok:false, reason:'parse_failed', host: host, snippet: snippet};
  }
  const pick = (p, r) => (p == null) ? null : {utilization: p, resets_at: r};
  return {ok:true, five_hour: pick(cur, curReset), seven_day: pick(wk, wkReset)};
} catch (e) { return {ok:false, reason:'exception', message: String(e)}; }"####;

const CODEX_USAGE_JS: &str = r####"try {
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
  // 세부 한도(코드 리뷰·모델별 추가 한도). 창 길이(limit_window_seconds/window_minutes)로
  // 5시간/주간을 가른다. 없으면 primary=5시간, secondary=주간으로 본다.
  const winSecs = (w) => num(w.limit_window_seconds) ?? (num(w.window_minutes) != null ? w.window_minutes * 60 : null);
  const buckets = [];
  const addLimit = (label, r) => {
    if (!label || !r) return;
    [[r.primary_window || r.primary, true], [r.secondary_window || r.secondary, false]].forEach(([w, dflt5h]) => {
      const v = win(w);
      if (!v) return;
      const secs = winSecs(w);
      buckets.push({label: label, utilization: v.utilization, resets_at: v.resets_at,
                    five_hour: secs != null ? secs <= 6 * 3600 : dflt5h});
    });
  };
  addLimit('Code Review', u.code_review_rate_limit || u.code_review_rate_limits);
  const pretty = (x) => String(x).replace(/[_-]+/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
  const extra = u.additional_rate_limits || u.additional_rate_limit || [];
  (Array.isArray(extra) ? extra : []).forEach(a => {
    if (!a) return;
    const name = a.limit_name || (a.metered_feature ? pretty(a.metered_feature) : null) || a.limit_id;
    addLimit(name, a.rate_limit || a.rate_limits || a);
  });
  return {ok:true, plan: plan, five_hour: primary, seven_day: secondary, model_buckets: buckets,
          keys: Object.keys(u)};
} catch (e) { return {ok:false, reason:'exception', message: String(e)}; }"####;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_and_lookup() {
        let ids: Vec<_> = ALL.iter().map(|s| s.id).collect();
        assert_eq!(ids, ["claude", "gemini", "codex"]);
        assert_eq!(spec("codex").unwrap().name, "Codex");
        assert!(spec("nope").is_none());
    }

    #[test]
    fn login_url_fallback() {
        assert_eq!(CLAUDE.effective_login_url(), "https://claude.ai/");
        assert!(GEMINI.effective_login_url().starts_with("https://accounts.google.com/ServiceLogin"));
        assert!(GEMINI.needs_rendered_dom && GEMINI.reload_before_fetch);
        assert!(!CLAUDE.needs_rendered_dom && !CODEX.needs_rendered_dom);
    }

    #[test]
    fn claude_js_markers() {
        let js = CLAUDE.usage_js;
        assert!(js.starts_with("try {\n  const orgsRes"));
        assert!(js.contains("'https://claude.ai/api/organizations'"));
        assert!(js.contains("max_20x") && js.contains("weekly_scoped"));
        assert!(js.contains("replace(/^default_claude_/,'')"));
        assert!(js.ends_with("catch (e) { return {ok:false, reason:'exception', message: String(e)}; }"));
    }

    #[test]
    fn gemini_js_markers_and_unescaping() {
        let js = GEMINI.usage_js;
        // Swift `\\` 는 JS에서 역슬래시 하나다.
        assert!(js.contains(r"/accounts\.google\.com/.test(host)"));
        assert!(js.contains(r"/(\d+(?:\.\d+)?)\s*%/"));
        assert!(!js.contains(r"\\"));
        // Swift `\"` 는 `"` 가 되므로 JS 문자열 안에 이스케이프 없는 따옴표가 온다.
        assert!(js.contains("'[data-test-id=\\\"gxu-currently\\\"]'"));
        assert!(js.contains(r#"[data-testid="current-usage"]"#));
        assert!(js.contains("reason:'parse_failed'"));
    }

    #[test]
    fn codex_js_markers() {
        let js = CODEX.usage_js;
        assert!(js.contains("backend-api/wham/usage"));
        assert!(js.contains("accounts/check/v4-2023-04-27"));
        assert!(js.contains("'ChatGPT-Account-ID'"));
        assert!(js.contains("secs <= 6 * 3600"));
        assert!(js.contains(r"replace(/\b\w/g, c => c.toUpperCase())"));
        assert!(!js.contains(r"\\"));
    }
}
