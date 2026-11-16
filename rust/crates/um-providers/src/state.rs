//! 조회 결과의 상태 반영 규칙. (Swift ProviderManager.swift `apply`, `subItems(for:)`, `remember`)
//!
//! 핵심: 조회 실패 != 로그아웃. 로그아웃 표시는 서버가 미로그인을 알려준 `AuthExpired`일 때만 한다.

use serde::{Deserialize, Serialize};
use um_core::model::{UsageSnapshot, UsageStatus, UsageSubItem};

/// 시작할 때 재시도 정책. 최대 4번, 2초 간격, 성공(Ok)하면 즉시 멈춘다. (ProviderManager.swift `refreshWithRetry`)
pub const START_RETRY_ATTEMPTS: u32 = 4;
pub const START_RETRY_INTERVAL_SECS: u64 = 2;

/// 시작 재시도를 계속해야 하는가(방금 결과가 Ok가 아니면 계속).
pub fn should_retry(latest: &UsageSnapshot) -> bool {
    latest.status != UsageStatus::Ok
}

/// 마지막으로 읽은 세부 항목(id·잔여율). 로그아웃·실패 중에도 설정 탭에서 쓰려고 기억한다.
/// (Swift `KnownItem`, UserDefaults "knownSubItems"에 저장하던 값)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownItem {
    pub id: String,
    pub ratio: f64,
}

/// `apply`가 낸 결과. 호출자가 상태에 적용한다.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// Some이면 저장된 스냅샷을 이 값으로 바꾼다. None이면 직전 값 유지.
    pub store: Option<UsageSnapshot>,
    /// Some이면 로그인 표시를 이 값으로 바꾼다. None이면 그대로.
    pub logged_in: Option<bool>,
    /// Some이면 세부 항목을 기억한다(Ok일 때만).
    pub remember: Option<Vec<UsageSubItem>>,
}

/// 새 결과를 직전 스냅샷에 대해 어떻게 반영할지 정한다. (ProviderManager.swift:76-93)
///
/// - Ok: 저장 + 로그인 표시 + 세부 항목 기억.
/// - AuthExpired: 저장 + 로그아웃 표시.
/// - Stale·Unavailable: 직전이 Ok면 무시(값 유지), 아니면 실패 상태를 그대로 저장.
pub fn apply(prev: Option<&UsageSnapshot>, new: UsageSnapshot) -> Outcome {
    match new.status {
        UsageStatus::Ok => {
            let items = new.sub_items();
            Outcome { store: Some(new), logged_in: Some(true), remember: Some(items) }
        }
        UsageStatus::AuthExpired => Outcome { store: Some(new), logged_in: Some(false), remember: None },
        UsageStatus::Stale | UsageStatus::Unavailable(_) => {
            if prev.is_some_and(|p| p.status == UsageStatus::Ok) {
                Outcome { store: None, logged_in: None, remember: None }
            } else {
                Outcome { store: Some(new), logged_in: None, remember: None }
            }
        }
    }
}

/// 한 AI의 현재 상태. (Swift `ProviderState`에서 웹 세션을 뺀 순수 부분)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProviderState {
    pub snapshot: Option<UsageSnapshot>,
    pub logged_in: bool,
    pub known_items: Vec<KnownItem>,
}

impl ProviderState {
    /// 결과를 반영한다. 기억한 세부 항목이 바뀌었으면 true(호출자가 저장).
    pub fn apply(&mut self, new: UsageSnapshot) -> bool {
        let out = apply(self.snapshot.as_ref(), new);
        if let Some(s) = out.store {
            self.snapshot = Some(s);
        }
        if let Some(l) = out.logged_in {
            self.logged_in = l;
        }
        match out.remember {
            Some(items) => {
                let known: Vec<KnownItem> = items.iter().map(|i| KnownItem { id: i.id.clone(), ratio: i.remaining_ratio }).collect();
                let changed = self.known_items != known;
                self.known_items = known;
                changed
            }
            None => false,
        }
    }

    /// 로그아웃: 로그인 표시와 스냅샷을 비운다(기억한 세부 항목은 유지). (ProviderManager.swift `logout`)
    pub fn logout(&mut self) {
        self.logged_in = false;
        self.snapshot = None;
    }

    /// 설정 탭에 보일 세부 항목: 지금 읽은 값이 Ok면 그것(live = true), 아니면 기억해 둔 마지막 값(live = false).
    pub fn sub_items(&self) -> (Vec<UsageSubItem>, bool) {
        if let Some(s) = self.snapshot.as_ref().filter(|s| s.status == UsageStatus::Ok) {
            return (s.sub_items(), true);
        }
        let items = self.known_items.iter().filter_map(|k| UsageSubItem::from_id(&k.id, k.ratio, None)).collect();
        (items, false)
    }

    /// 오버레이·메뉴에 쓸 수 있는 정상 스냅샷(로그인 + Ok). (Swift `active`)
    pub fn active_snapshot(&self) -> Option<&UsageSnapshot> {
        self.snapshot.as_ref().filter(|s| self.logged_in && s.status == UsageStatus::Ok)
    }
}
