//! 이력(SQLite) + 분석 계산 테스트.
//! 골든: `tools/swift-ref/analytics_ref.swift`가 같은 history.json에서 Swift 코드로 뽑은 값
//! (`fixtures/analytics_ref.json`)과 비교한다. 개수는 정확히, 실수는 1e-9, Unix 시각은 1e-5초 허용
//! (Swift는 2001 기준 초로 계산해 Unix 초(약 1.8e9)의 반올림 오차 ~2e-7이 끼어든다).

use std::path::PathBuf;
use std::time::Instant;

use chrono::FixedOffset;
use serde_json::Value;
use um_core::analytics::{
    self, AnalyticsRange, DOWNSAMPLE_BUCKETS, Series, bucket_start, bucket_sums, busiest_hour,
    downsample, heat_cells, hour_sums, kpi_days, merge_item_order, num, stats, times,
    total_consumed, total_limit_hits,
};
use um_core::history::{HistoryDb, Nested, Ratios};

const PROVIDERS: [&str; 3] = ["claude", "gemini", "codex"];

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn seoul() -> FixedOffset {
    FixedOffset::east_opt(9 * 3600).unwrap()
}

fn close(a: f64, b: f64, tol: f64, what: &str) {
    assert!(
        (a - b).abs() <= tol,
        "{what}: rust {a} vs swift {b} (diff {})",
        (a - b).abs()
    );
}

fn opt_close(a: Option<f64>, b: &Value, tol: f64, what: &str) {
    match (a, b.as_f64()) {
        (Some(x), Some(y)) => close(x, y, tol, what),
        (None, None) => assert!(b.is_null(), "{what}: swift {b}"),
        (x, y) => panic!("{what}: rust {x:?} vs swift {y:?}"),
    }
}

fn imported() -> HistoryDb {
    let mut db = HistoryDb::open_in_memory().unwrap();
    db.import_swift_history_json(fixture("history.json"))
        .unwrap();
    db
}

fn check_series(
    s: &Series,
    down_since: f64,
    now: f64,
    range: AnalyticsRange,
    want: &Value,
    what: &str,
) {
    let tz = seoul();
    assert_eq!(
        s.points.len() as u64,
        want["points"].as_u64().unwrap(),
        "{what} points"
    );
    assert_eq!(
        s.events.len() as u64,
        want["events"].as_u64().unwrap(),
        "{what} events"
    );
    close(
        s.consumed(),
        want["consumed"].as_f64().unwrap(),
        1e-9,
        &format!("{what} consumed"),
    );
    let down = downsample(&s.points, down_since, now, DOWNSAMPLE_BUCKETS);
    assert_eq!(
        down.len() as u64,
        want["downsampled"].as_u64().unwrap(),
        "{what} downsampled"
    );
    close(
        down.iter().fold(0.0, |a, p| a + p.r),
        want["downsampled_sum_r"].as_f64().unwrap(),
        1e-9,
        &format!("{what} downsampled sum"),
    );
    let st = stats(s, now);
    let w = &want["stats"];
    opt_close(st.current, &w["current"], 1e-9, &format!("{what} current"));
    close(
        st.consumed,
        w["consumed"].as_f64().unwrap(),
        1e-9,
        &format!("{what} st.consumed"),
    );
    assert_eq!(
        u64::from(st.resets),
        w["resets"].as_u64().unwrap(),
        "{what} resets"
    );
    assert_eq!(
        u64::from(st.limit_hits),
        w["limit_hits"].as_u64().unwrap(),
        "{what} limit_hits"
    );
    opt_close(
        st.window_use,
        &w["window_use"],
        1e-9,
        &format!("{what} window_use"),
    );
    opt_close(
        st.rate_per_hour,
        &w["rate_per_hour"],
        1e-9,
        &format!("{what} rate"),
    );
    for (h, v) in hour_sums(&tz, &s.events).iter().enumerate() {
        close(
            *v,
            want["hour_sums"][h].as_f64().unwrap(),
            1e-9,
            &format!("{what} hour {h}"),
        );
    }
    let bs = bucket_sums(&tz, range, &s.events);
    let wb = want["bucket_sums"].as_array().unwrap();
    assert_eq!(bs.len(), wb.len(), "{what} bucket count");
    for (a, b) in bs.iter().zip(wb) {
        close(
            a.0,
            b[0].as_f64().unwrap(),
            1e-5,
            &format!("{what} bucket t"),
        );
        close(
            a.1,
            b[1].as_f64().unwrap(),
            1e-9,
            &format!("{what} bucket sum"),
        );
    }
    let bh = busiest_hour(&tz, [s]).map(f64::from);
    opt_close(bh, &want["busiest_hour"], 0.0, &format!("{what} busiest"));
}

#[test]
fn golden_matches_swift() {
    let want: Value =
        serde_json::from_slice(&std::fs::read(fixture("analytics_ref.json")).unwrap()).unwrap();
    let t0 = Instant::now();
    let db = imported();
    eprintln!("import: {:?}", t0.elapsed());
    assert_eq!(
        db.sample_count().unwrap() as u64,
        want["sample_count"].as_u64().unwrap()
    );

    let now = want["now"].as_f64().unwrap();
    opt_close(db.earliest().unwrap(), &want["earliest"], 1e-5, "earliest");
    opt_close(
        db.earliest_items().unwrap(),
        &want["earliest_items"],
        1e-5,
        "earliest_items",
    );

    for p in PROVIDERS {
        let w = &want["pace"][p];
        match db.pace(p, now).unwrap() {
            Some(pc) => {
                close(
                    pc.rate_per_hour,
                    w["rate_per_hour"].as_f64().unwrap(),
                    1e-9,
                    &format!("pace {p} rate"),
                );
                close(
                    pc.depletion,
                    w["depletion"].as_f64().unwrap(),
                    1e-5,
                    &format!("pace {p} depletion"),
                );
            }
            None => assert!(w.is_null(), "pace {p}: swift {w}"),
        }
    }

    let tz = seoul();
    let t1 = Instant::now();
    for range in AnalyticsRange::ALL {
        let key = format!("{range:?}").to_lowercase();
        let r = &want["ranges"][key.as_str()];
        assert!(r.is_object(), "range {key}");
        let since = range.since(now);
        let mut ai: Vec<Series> = Vec::new();
        let mut items: Vec<Series> = Vec::new();
        for p in PROVIDERS {
            let what = format!("{key}/{p}");
            let w = &r["providers"][p];
            let pts = db.series(p, since).unwrap();
            assert_eq!(
                pts.len() as u64,
                w["series_points"].as_u64().unwrap(),
                "{what} series_points"
            );
            if pts.len() >= 2 {
                let s = Series {
                    points: pts,
                    events: db.consumption_events(p, since).unwrap(),
                };
                check_series(&s, since, now, range, &w["ai"], &format!("{what}/ai"));
                ai.push(s);
            } else {
                assert!(w["ai"].is_null(), "{what} ai should be null");
            }
            let ids = db.item_ids(p, since).unwrap();
            let want_ids: Vec<&str> = w["item_ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            assert_eq!(ids, want_ids, "{what} item_ids");
            // 지금 스냅샷이 없으므로 순서 = 이력 순서.
            for k in merge_item_order(&[], &ids) {
                let pts = db.item_series(p, &k, since).unwrap();
                if pts.is_empty() {
                    assert!(w["items"].get(&k).is_none());
                    continue;
                }
                let s = Series {
                    events: analytics::drops(&pts),
                    points: pts,
                };
                check_series(
                    &s,
                    since,
                    now,
                    range,
                    &w["items"][&k],
                    &format!("{what}/{k}"),
                );
                items.push(s);
            }
        }
        let kpi = &r["kpi"];
        let total = total_consumed(&ai);
        let days = kpi_days(range);
        close(days, kpi["days"].as_f64().unwrap(), 0.0, "days");
        close(
            total,
            kpi["total"].as_f64().unwrap(),
            1e-9,
            &format!("{key} total"),
        );
        close(
            total / days,
            kpi["per_day"].as_f64().unwrap(),
            1e-9,
            &format!("{key} per_day"),
        );
        assert_eq!(
            u64::from(total_limit_hits(&items, now)),
            kpi["limit_hits"].as_u64().unwrap(),
            "{key} hits"
        );
        assert_eq!(times(total), kpi["total_text"].as_str().unwrap());
        assert_eq!(times(total / days), kpi["per_day_text"].as_str().unwrap());
        opt_close(
            busiest_hour(&tz, &ai).map(f64::from),
            &kpi["busiest_hour"],
            0.0,
            &format!("{key} kpi busiest"),
        );

        let heat = heat_cells(&tz, 0, &ai);
        let wd: Vec<u64> = r["heat"]["weekdays"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        assert_eq!(
            heat.weekdays
                .iter()
                .map(|&d| u64::from(d))
                .collect::<Vec<_>>(),
            wd
        );
        let wc = r["heat"]["cells"].as_array().unwrap();
        assert_eq!(heat.cells.len(), wc.len());
        for (i, (c, w)) in heat.cells.iter().zip(wc).enumerate() {
            close(
                c.amount,
                w.as_f64().unwrap(),
                1e-9,
                &format!("{key} heat {i}"),
            );
        }
        close(
            bucket_start(&tz, range, since),
            r["bucket_start_since"].as_f64().unwrap(),
            1e-5,
            "bucket since",
        );
        close(
            bucket_start(&tz, range, now),
            r["bucket_start_now"].as_f64().unwrap(),
            1e-5,
            "bucket now",
        );
    }
    eprintln!(
        "analytics (4 ranges x 3 providers + items): {:?}",
        t1.elapsed()
    );

    for f in want["format"].as_array().unwrap() {
        let v = f["v"].as_f64().unwrap();
        assert_eq!(times(v), f["times"].as_str().unwrap(), "times({v})");
        assert_eq!(num(v), f["num"].as_str().unwrap(), "num({v})");
    }
}

#[test]
fn import_is_idempotent() {
    let mut db = HistoryDb::open_in_memory().unwrap();
    let a = db
        .import_swift_history_json(fixture("history.json"))
        .unwrap();
    assert_eq!(a.samples, 11795);
    assert!(a.rows_inserted > a.samples);
    let n = db.sample_count().unwrap();
    let b = db
        .import_swift_history_json(fixture("history.json"))
        .unwrap();
    assert_eq!((b.samples, b.rows_inserted), (11795, 0));
    assert_eq!(db.sample_count().unwrap(), n);
}

fn r(pairs: &[(&str, f64)]) -> Ratios {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

fn n(pairs: &[(&str, &[(&str, f64)])]) -> Nested {
    pairs.iter().map(|(k, m)| (k.to_string(), r(m))).collect()
}

#[test]
fn record_rules() {
    let mut db = HistoryDb::open_in_memory().unwrap();
    let none = Nested::new();
    let items = n(&[("claude", &[("5h", 0.5), ("weekly", 0.9)])]);
    // 활성 없음 -> 기록 안 함.
    assert!(!db.record(1000.0, &Ratios::new(), &none, &none).unwrap());
    // 첫 기록.
    assert!(
        db.record(1000.0, &r(&[("claude", 0.5)]), &none, &items)
            .unwrap()
    );
    // 변화 없고 45초 미만 -> 건너뜀. 0.0005 이하 변화도 변화 아님.
    assert!(
        !db.record(1010.0, &r(&[("claude", 0.5004)]), &none, &items)
            .unwrap()
    );
    // 변화 > 0.0005 -> 즉시.
    assert!(
        db.record(1011.0, &r(&[("claude", 0.49)]), &none, &items)
            .unwrap()
    );
    // 새 항목 키 -> 즉시.
    let items2 = n(&[("claude", &[("5h", 0.49), ("weekly", 0.9), ("model:X", 0.3)])]);
    assert!(
        db.record(1012.0, &r(&[("claude", 0.49)]), &none, &items2)
            .unwrap()
    );
    // 새 AI -> 즉시.
    assert!(
        db.record(
            1013.0,
            &r(&[("claude", 0.49), ("codex", 1.0)]),
            &none,
            &items2
        )
        .unwrap()
    );
    // 변화 없음, 45초 이상 -> 기록.
    assert!(
        !db.record(
            1050.0,
            &r(&[("claude", 0.49), ("codex", 1.0)]),
            &none,
            &items2
        )
        .unwrap()
    );
    assert!(
        db.record(
            1058.0,
            &r(&[("claude", 0.49), ("codex", 1.0)]),
            &none,
            &items2
        )
        .unwrap()
    );
    assert_eq!(db.sample_count().unwrap(), 5);
    // 재시작해도(같은 DB 다시 열기) 직전 표본과 비교한다: 파일 DB로 확인.
    let dir = std::env::temp_dir().join(format!("um-hist-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("h.db");
    {
        let mut f = HistoryDb::open(&path).unwrap();
        assert!(
            f.record(1.0e9, &r(&[("claude", 0.5)]), &none, &none)
                .unwrap()
        );
    }
    let mut f = HistoryDb::open(&path).unwrap();
    // last_record는 프로세스별이라(Swift와 같다) 시작 직후 첫 호출은 변화가 없어도 기록된다.
    assert!(
        f.record(1.0e9 + 1.0, &r(&[("claude", 0.5)]), &none, &none)
            .unwrap()
    );
    assert!(
        !f.record(1.0e9 + 2.0, &r(&[("claude", 0.5)]), &none, &none)
            .unwrap()
    );
    drop(f);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn retention_prunes_old_rows() {
    let mut db = HistoryDb::open_in_memory().unwrap();
    let none = Nested::new();
    let day = 86400.0;
    assert!(
        db.record(10.0 * day, &r(&[("claude", 1.0)]), &none, &none)
            .unwrap()
    );
    assert_eq!(db.earliest().unwrap(), Some(10.0 * day));
    // 35일 넘게 지난 뒤 기록하면 오래된 행이 지워진다.
    assert!(
        db.record(
            10.0 * day + 36.0 * day,
            &r(&[("claude", 0.9)]),
            &none,
            &none
        )
        .unwrap()
    );
    assert_eq!(db.earliest().unwrap(), Some(46.0 * day));
}

#[test]
fn legacy_item_fallback_and_consumption() {
    let mut db = HistoryDb::open_in_memory().unwrap();
    let none = Nested::new();
    // 구버전: 항목 없음. gemini는 버킷 있음(gemini-5h), claude는 없음.
    let b1 = n(&[("gemini", &[("gemini-5h", 0.8), ("w", 0.9)])]);
    let b2 = n(&[("gemini", &[("gemini-5h", 0.7), ("w", 0.5)])]);
    db.record(100.0, &r(&[("gemini", 0.9), ("claude", 0.9)]), &b1, &none)
        .unwrap();
    db.record(200.0, &r(&[("gemini", 0.9), ("claude", 0.8)]), &b2, &none)
        .unwrap();
    // 신버전: 항목 있음.
    let it = n(&[
        ("claude", &[("5h", 0.6), ("weekly", 0.9)]),
        ("gemini", &[("5h", 0.7)]),
    ]);
    db.record(300.0, &r(&[("gemini", 0.9), ("claude", 0.6)]), &b2, &it)
        .unwrap();

    // gemini 5h: 버킷 gemini-5h 0.8, 0.7 -> 항목 0.7
    let g: Vec<f64> = db
        .item_series("gemini", "5h", 0.0)
        .unwrap()
        .iter()
        .map(|p| p.r)
        .collect();
    assert_eq!(g, vec![0.8, 0.7, 0.7]);
    // claude 5h: 대표 잔여율 0.9, 0.8 -> 항목 0.6
    let c: Vec<f64> = db
        .item_series("claude", "5h", 0.0)
        .unwrap()
        .iter()
        .map(|p| p.r)
        .collect();
    assert_eq!(c, vec![0.9, 0.8, 0.6]);
    // weekly는 신버전 표본만.
    assert_eq!(db.item_series("claude", "weekly", 0.0).unwrap().len(), 1);
    // 항목 id 순서: 구버전 "5h"가 먼저, 그다음 신버전 키(이름순).
    assert_eq!(db.item_ids("claude", 0.0).unwrap(), vec!["5h", "weekly"]);
    assert_eq!(db.item_ids("claude", 250.0).unwrap(), vec!["5h", "weekly"]);
    assert_eq!(db.earliest_items().unwrap(), Some(300.0));

    // 소비: gemini 대표 0.9 -> 0.9 -> 0.9 이지만 버킷 w가 0.9 -> 0.5 로 0.4 하락.
    let ev = db.consumption_events("gemini", 0.0).unwrap();
    assert_eq!(ev.len(), 1);
    assert!((ev[0].amount - 0.4).abs() < 1e-12 && ev[0].t == 200.0);
    // claude: 0.9 -> 0.8 (0.1), 0.8 -> 0.6 (0.2)
    let ev = db.consumption_events("claude", 0.0).unwrap();
    assert_eq!(ev.len(), 2);
    assert!((ev[1].amount - 0.2).abs() < 1e-12);

    // 소진 예측: gemini는 하락 없음 -> None. claude: 200초(0.0556h) 동안 0.3 하락.
    assert_eq!(db.pace("gemini", 300.0).unwrap(), None);
    let p = db.pace("claude", 300.0).unwrap().unwrap();
    assert!((p.rate_per_hour - 0.3 / (200.0 / 3600.0)).abs() < 1e-9);
    // 구간이 0.05h(180초) 이하면 None.
    assert_eq!(
        db.pace("claude", 100.0 + 3.0 * 3600.0 + 150.0).unwrap(),
        None
    );
}

#[test]
fn pace_uses_three_hour_window() {
    let mut db = HistoryDb::open_in_memory().unwrap();
    let none = Nested::new();
    let base = 1.0e9;
    for (i, v) in [1.0, 0.9, 0.8].iter().enumerate() {
        db.record(
            base + i as f64 * 1800.0,
            &r(&[("claude", *v)]),
            &none,
            &none,
        )
        .unwrap();
    }
    // 1시간 동안 0.2 하락 = 시간당 0.2, 남은 0.8 -> 4시간 뒤 바닥.
    let now = base + 3600.0;
    let p = db.pace("claude", now).unwrap().unwrap();
    assert!((p.rate_per_hour - 0.2).abs() < 1e-12);
    assert!((p.depletion - (now + 4.0 * 3600.0)).abs() < 1e-6);
}
