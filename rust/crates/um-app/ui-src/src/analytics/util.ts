// 분석 창 공용 타입·숫자 서식(Swift AnalyticsView.swift 하단 num/times/ppText 대응).
export type Range = 'day1' | 'day3' | 'week' | 'month';
export const RANGES: { id: Range; key: string }[] = [
  { id: 'day1', key: 'range.day1' }, { id: 'day3', key: 'range.day3' },
  { id: 'week', key: 'range.week' }, { id: 'month', key: 'range.month' },
];

export interface Series {
  provider: string; item: string | null; name: string; color: string; consumed: number;
  hours: number[]; remaining: [number, number][]; buckets: [number, number][];
}
export interface Eta { kind: 'safe' | 'at' | 'idle'; t?: number }
export interface Row {
  name: string; color: string; current: number | null; consumed: number; resets: number;
  windowUse: number | null; limitHits: number; ratePerHour: number | null; eta: Eta;
}
export interface Group { id: string; name: string; color: string; rows: Row[] }
export interface Data {
  now: number; since: number; bucketSeconds: number; earliest: number | null;
  hasData: string[]; providers: { id: string; name: string; color: string }[];
  scopedItems: { id: string; name: string; color: string }[]; itemMode: boolean;
  kpi: { total: string; dailyAvg: string; busiest: number | null; hits: number };
  series: Series[];
  heat: { weekdays: number[]; cells: [number, number, number][] };
  detail: Group[];
}

/** 10 이상은 정수, 그 아래는 소수 한 자리(Swift `num`). */
export const num = (v: number) => (v >= 10 ? v.toFixed(0) : v.toFixed(1));
/** 회분 표기(Swift `times`). KPI 합계는 Rust가 문자열로 주고, 도넛 툴팁에서만 쓴다. */
export const times = (v: number) => (v < 1 ? v.toFixed(2) : v.toFixed(1));

const p2 = (n: number) => String(n).padStart(2, '0');
export const fmtHM = (d: Date) => `${p2(d.getHours())}:${p2(d.getMinutes())}`;
export const fmtMD = (d: Date) => `${d.getMonth() + 1}/${d.getDate()}`;
export const fmtYMD = (d: Date) => `${d.getFullYear()}/${d.getMonth() + 1}/${d.getDate()}`;

/** 눈금 간격이 1·2·2.5·5 x 10^k 가 되게 맞춘 y축 눈금. */
export function niceTicks(max: number, count = 4): { ticks: number[]; top: number } {
  if (!(max > 0)) return { ticks: [0, 1], top: 1 };
  const raw = max / count;
  const mag = Math.pow(10, Math.floor(Math.log10(raw)));
  const step = [1, 2, 2.5, 5, 10].map((m) => m * mag).find((s) => s >= raw - 1e-12)!;
  const top = Math.ceil(max / step - 1e-9) * step;
  const ticks: number[] = [];
  for (let v = 0; v <= top + step / 2; v += step) ticks.push(+v.toFixed(6));
  return { ticks, top };
}
export const fmtTick = (v: number) => (Number.isInteger(v) ? String(v) : v.toFixed(1));

const HOUR = 3600, DAY = 86400;
/** 로컬 시각에 맞춘 시간축 눈금(잔여량 추이 x축). 하루 미만 간격은 시각, 이상은 날짜. */
export function timeTicks(since: number, now: number, target = 6): { t: number; label: string }[] {
  const span = now - since;
  const steps = [HOUR, 2 * HOUR, 3 * HOUR, 6 * HOUR, 12 * HOUR, DAY, 2 * DAY, 3 * DAY, 7 * DAY];
  const step = steps.find((s) => span / s <= target) ?? 7 * DAY;
  const out: { t: number; label: string }[] = [];
  const d = new Date(since * 1000);
  if (step < DAY) {
    const h = step / HOUR;
    d.setMinutes(0, 0, 0);
    if (d.getTime() / 1000 < since) d.setHours(d.getHours() + 1);
    while (d.getHours() % h !== 0) d.setHours(d.getHours() + 1);
    for (; d.getTime() / 1000 <= now; d.setHours(d.getHours() + h)) {
      out.push({ t: d.getTime() / 1000, label: d.getHours() === 0 ? fmtMD(d) : fmtHM(d) });
    }
  } else {
    d.setHours(0, 0, 0, 0);
    if (d.getTime() / 1000 < since) d.setDate(d.getDate() + 1);
    for (; d.getTime() / 1000 <= now; d.setDate(d.getDate() + step / DAY)) {
      out.push({ t: d.getTime() / 1000, label: fmtMD(d) });
    }
  }
  return out;
}

/** 이벤트 시각 -> 버킷 시작(로컬 자정 기준). Rust `bucket_start` 와 같은 규칙. */
export function bucketStart(t: number, step: number): number {
  const d = new Date(t * 1000);
  const day = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime() / 1000;
  return step >= DAY ? day : day + Math.floor((t - day) / step) * step;
}
