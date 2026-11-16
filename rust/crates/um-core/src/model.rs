//! 사용량 공통 모델. (Swift Provider.swift)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 조회 상태. (Swift `UsageStatus`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UsageStatus {
    Ok,
    /// 정의만 있고 Swift에서도 쓰이지 않는다(계획서 부록 C).
    Stale,
    AuthExpired,
    Unavailable(String),
}

/// 모델별 한도 버킷. (Swift `UsageModelBucket`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageModelBucket {
    pub label: String,
    pub remaining_ratio: f64,
    pub reset_at: Option<DateTime<Utc>>,
    pub is_five_hour: bool,
}

/// 한 AI의 사용량 스냅샷. (Swift `UsageSnapshot`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageSnapshot {
    pub id: String,
    pub remaining_ratio: f64,
    pub secondary_ratio: Option<f64>,
    pub opus_ratio: Option<f64>,
    pub reset_at: Option<DateTime<Utc>>,
    pub secondary_reset_at: Option<DateTime<Utc>>,
    pub opus_reset_at: Option<DateTime<Utc>>,
    pub status: UsageStatus,
    pub last_updated: DateTime<Utc>,
    pub plan: Option<String>,
    pub model_buckets: Option<Vec<UsageModelBucket>>,
    pub five_hour_ratio: Option<f64>,
    pub five_hour_reset_at: Option<DateTime<Utc>>,
    pub raw_buckets: Option<std::collections::BTreeMap<String, f64>>,
}

impl UsageSnapshot {
    /// 실패 상태 스냅샷(값 없음). (Swift `WebSession.snapshot(_:_:)`)
    pub fn failed(id: &str, status: UsageStatus, now: DateTime<Utc>) -> Self {
        Self {
            id: id.to_string(),
            remaining_ratio: 0.0,
            secondary_ratio: None,
            opus_ratio: None,
            reset_at: None,
            secondary_reset_at: None,
            opus_reset_at: None,
            status,
            last_updated: now,
            plan: None,
            model_buckets: None,
            five_hour_ratio: None,
            five_hour_reset_at: None,
            raw_buckets: None,
        }
    }

    /// 표시 순서: 대표 5시간 → 대표 주간 → 모델 버킷(서버 순서). (Swift `subItems`)
    pub fn sub_items(&self) -> Vec<UsageSubItem> {
        let mut out = vec![UsageSubItem {
            id: "5h".into(),
            model_label: None,
            is_five_hour: true,
            remaining_ratio: self.five_hour_ratio.unwrap_or(self.remaining_ratio),
            reset_at: self.five_hour_reset_at.or(self.reset_at),
        }];
        if let Some(w) = self.secondary_ratio {
            out.push(UsageSubItem {
                id: "weekly".into(),
                model_label: None,
                is_five_hour: false,
                remaining_ratio: w,
                reset_at: self.secondary_reset_at,
            });
        }
        for b in self.model_buckets.iter().flatten() {
            out.push(UsageSubItem {
                id: format!("model:{}:{}", b.label, if b.is_five_hour { "5h" } else { "weekly" }),
                model_label: Some(b.label.clone()),
                is_five_hour: b.is_five_hour,
                remaining_ratio: b.remaining_ratio,
                reset_at: b.reset_at,
            });
        }
        out
    }
}

/// 한 AI의 세부 항목(5시간·주간·모델별). (Swift `UsageSubItem`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageSubItem {
    /// "5h" · "weekly" · "model:<이름>:5h" · "model:<이름>:weekly"
    pub id: String,
    pub model_label: Option<String>,
    pub is_five_hour: bool,
    pub remaining_ratio: f64,
    pub reset_at: Option<DateTime<Utc>>,
}

impl UsageSubItem {
    /// 저장된 id만으로 항목을 되살린다. 형식이 틀리면 None. (Swift `init?(id:remainingRatio:resetAt:)`)
    pub fn from_id(id: &str, remaining_ratio: f64, reset_at: Option<DateTime<Utc>>) -> Option<Self> {
        if id == "5h" || id == "weekly" {
            return Some(Self { id: id.into(), model_label: None, is_five_hour: id == "5h", remaining_ratio, reset_at });
        }
        let rest = id.strip_prefix("model:")?;
        let colon = rest.rfind(':')?;
        let label = &rest[..colon];
        if label.is_empty() {
            return None;
        }
        Some(Self {
            id: id.into(),
            model_label: Some(label.into()),
            is_five_hour: &rest[colon + 1..] == "5h",
            remaining_ratio,
            reset_at,
        })
    }

    /// 설정 키 "provider/itemId". (Swift `OverlaySettings.subItemKey`)
    pub fn settings_key(provider_id: &str, item_id: &str) -> String {
        format!("{provider_id}/{item_id}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_item_from_id() {
        let a = UsageSubItem::from_id("model:Claude·GPT (agy):5h", 0.5, None).unwrap();
        assert_eq!(a.model_label.as_deref(), Some("Claude·GPT (agy)"));
        assert!(a.is_five_hour);
        let b = UsageSubItem::from_id("model:Fable:weekly", 0.5, None).unwrap();
        assert!(!b.is_five_hour);
        assert!(UsageSubItem::from_id("weekly", 1.0, None).is_some());
        assert!(UsageSubItem::from_id("model::5h", 1.0, None).is_none());
        assert!(UsageSubItem::from_id("bogus", 1.0, None).is_none());
    }
}
