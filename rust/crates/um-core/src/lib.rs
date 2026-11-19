//! UsageMeter 핵심 로직. UI·OS 의존 코드를 넣지 않는다(계획서 3장).
//!
//! Swift 원본과 1:1로 대응하는 모듈:
//! - `types`      OverlaySettings.swift의 열거형(BorderEdge, SegPart …)
//! - `model`      Provider.swift(UsageSnapshot, UsageSubItem)
//! - `settings`   OverlaySettings.swift(SettingsSnapshot, 기본값, 마이그레이션) + UserDefaults 가져오기
//! - `segments`   OverlaySettings.swift의 세그먼트 그래프 함수
//! - `i18n`       Localization.swift
//! - `history`    HistoryStore.swift (SQLite로 저장)
//! - `analytics`  AnalyticsModel.swift + AnalyticsView.swift의 계산
//! - `notify_rules` NotificationManager.swift의 임계값 판정
//! - `update`     UpdateChecker.swift의 버전 비교

pub mod analytics;
pub mod dock;
pub mod history;
pub mod hover;
pub mod i18n;
pub mod menu_text;
pub mod model;
pub mod notify_rules;
pub mod segments;
pub mod settings;
pub mod types;
pub mod update;
