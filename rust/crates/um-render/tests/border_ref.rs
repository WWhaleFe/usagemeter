//! Swift `BorderView`(ImageRenderer)가 뽑은 기준 이미지와 Rust 렌더를 픽셀 단위로 비교한다.
//! 기준 이미지 생성: `rust/tools/border-fixtures/build.sh run`. 입력: `rust/tools/border-fixtures/cases.json`.
//! 비교는 흰 바탕에 합성한 뒤 채널별 절댓값 차. 경계선 안티앨리어싱 차이는 허용하고 구조 차이만 잡는다.
//! 통계를 보려면 `cargo test -p um-render --test border_ref -- --nocapture`.
//! 실패하면(또는 `BORDER_DUMP=1`이면) Rust 렌더와 차이 이미지를 `<target>/border-diff/`에 쓴다.

use std::path::{Path, PathBuf};

use chrono::DateTime;
use serde_json::Value;
use tiny_skia::Pixmap;
use um_core::model::{UsageModelBucket, UsageSnapshot, UsageStatus};
use um_core::settings::{Settings, SettingsSnapshot};
use um_render::border::{BorderInput, ProviderBand, build_paths, provider_color, render, sub_item_bar_parts};

const SCALE: f64 = 2.0;
// 임계값 근거: 55개 케이스 실측 최댓값은 mean_all 0.072, mean_ink 1.29, 차>32 픽셀 148개(전체의 0.015%)로,
// 전부 선 가장자리 안티앨리어싱·길이 비율 환산(SwiftUI trim vs kurbo) 오차다. 모서리 하나가 빠지거나 굵기
// 구간이 어긋나는 구조 차이는 수백~1만 개 픽셀을 넘긴다(수정 전 실제 사례: 1.2만 개). 실측의 약 2~3배로 잡는다.
/// 전체 픽셀 평균 채널 차 상한(0~255). 얇은 선 위주라 대부분 0이므로 값이 작다.
const MAX_MEAN_ALL: f64 = 0.15;
/// 선이 있는(어느 한쪽이라도 알파 > 0) 픽셀 평균 채널 차 상한.
const MAX_MEAN_INK: f64 = 2.5;
/// 채널 차가 32를 넘는 픽셀의 전체 대비 비율 상한(구조 차이 판정). 요구 기준 0.5%보다 10배 엄격.
const MAX_BIG_FRAC: f64 = 0.0005;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn cases() -> Vec<Value> {
    let p = manifest().join("../../tools/border-fixtures/cases.json");
    serde_json::from_str(&std::fs::read_to_string(p).expect("cases.json")).expect("cases.json 파싱")
}

fn f(v: &Value) -> f64 {
    v.as_f64().expect("number")
}

/// ais 항목 -> 스냅샷(세부 항목 원천 값 포함). tools/border-fixtures/main.swift `makeSnapshot`과 같은 규칙.
fn snapshot_of(ai: &Value) -> UsageSnapshot {
    let id = ai["id"].as_str().unwrap();
    let now = DateTime::from_timestamp(0, 0).unwrap();
    let mut s = UsageSnapshot::failed(id, UsageStatus::Ok, now);
    s.remaining_ratio = f(&ai["ratio"]);
    s.secondary_ratio = ai.get("weekly").map(f);
    s.five_hour_ratio = ai.get("fiveHour").map(f);
    let models: Vec<UsageModelBucket> = ai
        .get("models")
        .and_then(|m| m.as_array())
        .into_iter()
        .flatten()
        .map(|m| UsageModelBucket {
            label: m["label"].as_str().unwrap().to_string(),
            remaining_ratio: f(&m["ratio"]),
            reset_at: None,
            is_five_hour: m.get("fiveHour").and_then(|b| b.as_bool()).unwrap_or(false),
        })
        .collect();
    s.model_buckets = if models.is_empty() { None } else { Some(models) };
    s
}

struct Stats {
    max: u8,
    mean_all: f64,
    mean_ink: f64,
    big: usize,
    ink: usize,
    total: usize,
}

/// premultiplied RGBA 한 픽셀을 흰 바탕에 합성한 RGB로.
fn over_white(px: &[u8]) -> [i32; 3] {
    let a = i32::from(px[3]);
    [0, 1, 2].map(|i| (i32::from(px[i]) + (255 - a)).min(255))
}

fn compare(rust: &Pixmap, reference: &Pixmap) -> (Stats, Vec<u8>) {
    assert_eq!((rust.width(), rust.height()), (reference.width(), reference.height()), "크기가 다름");
    let (a, b) = (rust.data(), reference.data());
    let total = (rust.width() * rust.height()) as usize;
    let mut diff_img = vec![255u8; total * 4];
    let (mut sum_all, mut sum_ink, mut ink, mut big, mut max) = (0u64, 0u64, 0usize, 0usize, 0u8);
    for i in 0..total {
        let (pa, pb) = (&a[i * 4..i * 4 + 4], &b[i * 4..i * 4 + 4]);
        let (ca, cb) = (over_white(pa), over_white(pb));
        let d: Vec<u8> = (0..3).map(|k| (ca[k] - cb[k]).unsigned_abs() as u8).collect();
        let m = *d.iter().max().unwrap();
        let s: u64 = d.iter().map(|&v| u64::from(v)).sum();
        sum_all += s;
        if pa[3] > 0 || pb[3] > 0 {
            ink += 1;
            sum_ink += s;
        }
        if m > 32 {
            big += 1;
        }
        max = max.max(m);
        // 차이 이미지: 흰 바탕에서 차이가 클수록 붉게(x4 증폭).
        let v = 255 - m.saturating_mul(4);
        diff_img[i * 4..i * 4 + 4].copy_from_slice(&[255, v, v, 255]);
    }
    let stats = Stats {
        max,
        mean_all: sum_all as f64 / (total * 3) as f64,
        mean_ink: if ink > 0 { sum_ink as f64 / (ink * 3) as f64 } else { 0.0 },
        big,
        ink,
        total,
    };
    (stats, diff_img)
}

fn diff_dir() -> PathBuf {
    let base = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| manifest().join("../../target"));
    base.join("border-diff")
}

fn dump(name: &str, rust: &Pixmap, diff: &[u8]) {
    let dir = diff_dir();
    std::fs::create_dir_all(&dir).unwrap();
    rust.save_png(dir.join(format!("{name}.rust.png"))).unwrap();
    let mut d = Pixmap::new(rust.width(), rust.height()).unwrap();
    // 이미 불투명 RGB이므로 premultiplied와 같다.
    d.data_mut().copy_from_slice(diff);
    d.save_png(dir.join(format!("{name}.diff.png"))).unwrap();
}

fn render_case(case: &Value) -> Pixmap {
    let snap: SettingsSnapshot = serde_json::from_value(case["settings"].clone()).expect("settings 디코딩");
    let settings = Settings::from_snapshot(&snap);
    let (w, h) = (f(&case["screen"][0]), f(&case["screen"][1]));
    let bands: Vec<ProviderBand> = case["ais"]
        .as_array()
        .unwrap()
        .iter()
        .map(|ai| {
            let id = ai["id"].as_str().unwrap();
            let snap = snapshot_of(ai);
            ProviderBand {
                id: id.to_string(),
                ratio: snap.remaining_ratio,
                color: provider_color(&settings, id),
                parts: sub_item_bar_parts(&settings, id, &snap),
            }
        })
        .collect();
    let scene = build_paths(&BorderInput { settings: &settings, screen_w: w, screen_h: h, bands });
    render(&scene, (0.0, 0.0), (w, h), SCALE)
}

#[test]
fn border_matches_swift_reference() {
    let dir = manifest().join("tests/fixtures/ref");
    let always = std::env::var_os("BORDER_DUMP").is_some();
    let mut failures = vec![];
    println!("{:<34} {:>4} {:>8} {:>8} {:>7} {:>7}", "case", "max", "mean_all", "mean_ink", ">32", "ink");
    for case in cases() {
        let name = case["name"].as_str().unwrap();
        let reference = Pixmap::load_png(Path::new(&dir).join(format!("{name}.png"))).expect("기준 PNG");
        let rust = render_case(&case);
        let (st, diff) = compare(&rust, &reference);
        println!(
            "{:<34} {:>4} {:>8.4} {:>8.3} {:>7} {:>7}",
            name, st.max, st.mean_all, st.mean_ink, st.big, st.ink
        );
        let big_frac = st.big as f64 / st.total as f64;
        let ok = st.mean_all < MAX_MEAN_ALL && st.mean_ink < MAX_MEAN_INK && big_frac < MAX_BIG_FRAC;
        if !ok || always {
            dump(name, &rust, &diff);
        }
        if !ok {
            failures.push(name.to_string());
        }
    }
    assert!(failures.is_empty(), "기준 이미지와 크게 다른 케이스: {failures:?} (차이 이미지: {})", diff_dir().display());
}
