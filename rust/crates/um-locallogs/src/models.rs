//! 모델 표시 이름. 내부 ID(`claude-opus-5-5`)를 사람이 읽는 이름(`Claude Opus 5.5`)으로 바꾼다.
//! 저장된 값은 그대로 두고 조회 단계에서만 쓴다.

const EFFORTS: [&str; 5] = ["minimal", "low", "medium", "high", "xhigh"];
const VARIANTS: [&str; 3] = ["n", "tiered", "trial"];

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// 토큰 하나의 표시 형태: `120b` -> `120B`, `3.8` -> `3.8`, `flash` -> `Flash`.
fn word(t: &str) -> String {
    let starts_digit = t.starts_with(|c: char| c.is_ascii_digit());
    if starts_digit && t.ends_with(|c: char| c.is_ascii_alphabetic()) {
        t.to_uppercase()
    } else if t.chars().all(|c| c.is_ascii_digit() || c == '.') {
        t.to_string()
    } else {
        capitalize(t)
    }
}

/// 모델 이름을 표시용으로 정리한다. 이미 사람이 읽는 이름이면 끝의 괄호 꼬리만 뗀다.
pub fn display_name(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return String::new();
    }
    if raw.contains(char::is_whitespace) || raw.chars().any(|c| c.is_uppercase()) {
        return match raw.strip_suffix(')').and_then(|s| s.rfind('(').map(|i| &s[..i])) {
            Some(head) if !head.trim().is_empty() => head.trim().to_string(),
            _ => raw.to_string(),
        };
    }
    if raw.starts_with('<') {
        return raw.to_string();
    }
    let mut toks: Vec<&str> = raw.split('-').filter(|t| !t.is_empty()).collect();
    while toks.len() > 2 {
        let last = toks[toks.len() - 1];
        let tail = (last.len() == 8 && is_digits(last)) || EFFORTS.contains(&last) || VARIANTS.contains(&last);
        if !tail {
            break;
        }
        toks.pop();
    }
    // 연속된 순수 숫자 토큰은 점으로 잇는다.
    let mut merged: Vec<String> = Vec::new();
    let mut prev_digits = false;
    for t in toks {
        let d = is_digits(t);
        if d && prev_digits {
            if let Some(l) = merged.last_mut() {
                l.push('.');
                l.push_str(t);
            }
        } else {
            merged.push(t.to_string());
        }
        prev_digits = d;
    }
    let mut parts: Vec<String> = Vec::new();
    let mut it = merged.into_iter().peekable();
    if it.peek().is_some_and(|f| f == "gpt") {
        it.next();
        let mut first = "GPT".to_string();
        if let Some(n) = it.next() {
            first.push('-');
            if n.len() <= 3 && n.chars().all(|c| c.is_ascii_alphabetic()) {
                first.push_str(&n.to_uppercase());
            } else {
                first.push_str(&word(&n));
            }
        }
        parts.push(first);
    }
    parts.extend(it.map(|t| word(&t)));
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_examples() {
        for (raw, want) in [
            ("claude-opus-5-5", "Claude Opus 5.5"),
            ("claude-fable-5-1", "Claude Fable 5.1"),
            ("claude-opus-5", "Claude Opus 5"),
            ("claude-sonnet-5-5-medium", "Claude Sonnet 5.5"),
            ("claude-opus-4-5-20251101", "Claude Opus 4.5"),
            ("claude-3-5-sonnet", "Claude 3.5 Sonnet"),
            ("opus", "Opus"),
            ("gpt-6.1-sol", "GPT-6.1 Sol"),
            ("gpt-6-luna", "GPT-6 Luna"),
            ("gpt-oss-120b-medium", "GPT-OSS 120B"),
            ("gpt-5.1-codex-max", "GPT-5.1 Codex Max"),
            ("gemini-3.8-flash-n", "Gemini 3.8 Flash"),
            ("gemini-3.8-flash-tiered", "Gemini 3.8 Flash"),
            ("gemini-4-argon-trial", "Gemini 4 Argon"),
            ("gemini-4-hydrogen", "Gemini 4 Hydrogen"),
            ("gemini-3.5-flash-lite", "Gemini 3.5 Flash Lite"),
            ("<synthetic>", "<synthetic>"),
            ("Gemini 4 Hydrogen (XHigh)", "Gemini 4 Hydrogen"),
            ("Gemini 4 Neon (High)", "Gemini 4 Neon"),
            ("  ", ""),
        ] {
            assert_eq!(display_name(raw), want, "{raw}");
        }
    }
}
