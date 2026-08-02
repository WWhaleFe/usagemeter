# UsageMeter

**한국어** → [README.md](README.md)

A macOS menu-bar overlay widget that shows your remaining **Claude · Gemini · Codex (ChatGPT plan)** usage as thin colored bands along the screen edges.
As you use up your quota, the band gets shorter — so you can tell how much is left at a glance, without opening anything.

## ⬇️ Download (macOS)

**[Get the latest release](https://github.com/WWhaleFe/usagemeter/releases/latest)** — download `UsageMeter-vX.X.X.zip` → unzip → drag the app into **Applications**.
Universal binary (Apple Silicon + Intel), macOS 14+.

> ⚠️ The app is not notarized yet, so the first launch requires **right-click → Open**. If it still won't open, go to System Settings → Privacy & Security and click "Open Anyway".

> 🪟 **A Windows version** is also in development — but it's expected to take a **considerable amount of time** to complete.

<br>

## ✨ Features

- **Screen-edge overlay** — a colored band fills to your remaining ratio and shrinks from the anchor as you use quota.
- **Multi AI** — Claude (coral) · Gemini (blue) · Codex (teal). Overlap them on the same rail or give each AI its own lines.
- **Screen partitions** — split the screen with a menu-bar line and a Dock line, then wrap any region (e.g. just the menu bar, or just the Dock). Adjust boundaries by dragging directly on screen.
- **Segment layout** — 4 horizontal lines (top / menu line / Dock line / bottom) + 3 vertical sections per side, picked by clicking an interactive diagram. Only connected chains/loops are allowed; one-click shape presets included.
- **Corner control** — per-zone corner radii (with linked-boundary toggle), curve horizontal line ends up/down, scoop (concave wrap) corners, notch wrap with auto-detection.
- **Overlap rendering** — identical shapes stack with the most urgent band on top; partial overlaps split the thickness into side-by-side lanes only where they actually overlap, with transitions smoothed over hundreds of points so width changes are imperceptible.
- **Menu bar** — remaining-ratio ring icon (pick the reference AI), per-AI % next to the icon, and a high-contrast dropdown info card (light/dark aware) with 5-hour · weekly · per-model-weekly remaining, reset countdown, depletion forecast, and a 24-hour mini chart (values shown on hover).
- **Plan & per-model usage** — a subscription-plan badge next to each AI (e.g. Claude `Max (5x)`), plus 5-hour (current session), weekly (all models), and per-model weekly (e.g. `Fable`) remaining. Only the buckets the server actually returns are shown.
- **Analytics dashboard** — open it via "View Analytics" from the menu/Settings. Consumption share by AI (donut), usage-by-hour pattern, and summary KPIs across a range (1 day · 3 days · week · month). Show all, some, or a single AI.
- **Appearance (light/dark/system)** — follow the device or pick manually; menus, settings, and hover panels follow the theme.
- **AI display order** — one shared order for the dropdown, hover, and the menu-bar %.
- **Alerts** — macOS notifications at 75 / 90 / 95% usage.
- **Convenience** — login tab for per-AI sign in/out, auto-refresh interval (1 min–2 h or custom), 10 named presets + default-state save, launch at login, Korean / English / Japanese UI. While a Settings/Analytics window is open, a Dock icon lets you bring it back.
- Every setting persists across restarts.

## 🧩 How it works

- Usage is read via **in-page `fetch` inside a logged-in WKWebView**, which passes Cloudflare naturally (plain curl/URLSession gets 403).
- Claude: `GET /api/organizations/{uuid}/usage` (5-hour · weekly + per-model `weekly_scoped` buckets from `limits`) plus the plan from `rate_limit_tier`. Gemini: `gemini.google.com/usage` DOM. Codex: the same internal endpoint the official dashboard uses (`backend-api/wham/usage`) via the chatgpt.com session.
- The overlay is a borderless transparent window (click-through, above the menu bar and regular apps) drawn imperatively with SwiftUI **Canvas**. It's built from **thin strip windows that wrap only the edges/lines and never cover the screen center (a "hollow frame")**, so it doesn't hide the purchase/install/auth buttons (including Touch ID) that apps like App Store · Music · TV · Books · System Settings · web Apple Pay show in the center (v1.2.2 — avoids macOS's overlay-blocking behavior). The border is modeled as a segment graph; overlapping spans are computed as trim ranges along the path and widths are blended with Gaussian smoothing.
- Cookies stay in the local `WKWebsiteDataStore.default()` only and are **never sent anywhere**.

## 🚀 Run (development)

Runs with just the Command Line Tools' Swift — no full Xcode needed. **macOS 14+**.

```sh
swift run UsageMeter
```

Sign in from the menu-bar icon → **AI Login / Logout** (or the Login tab in Settings) and the bands appear.

## 📦 Package as an app

```sh
./build-app.sh          # builds a universal UsageMeter.app (icon = icon.png)
open UsageMeter.app
```

- Menu-bar-only app (no Dock icon, LSUIElement).
- Universal binary: arm64 + x86_64 combined with `lipo` (no Xcode required).
- Ad-hoc signed, so the first launch may require **right-click → Open**.
- The `.app` uses a separate cookie/settings store from `swift run`, so sign in again inside the app.

## 🗂 Project layout

```
usagemeter/
  Package.swift
  build-app.sh                  .app packaging (universal, icon.png → .icns)
  icon.png                      app icon source
  Sources/UsageMeter/
    main.swift / AppDelegate.swift
    OverlayWindow.swift         transparent, always-on-top, click-through window
    BorderView.swift            Canvas renderer + SegmentChainShape (segment graph)
    OverlaySettings.swift       shared settings (persisted)
    Localization.swift          ko/en/ja strings
    Providers.swift / WebSession.swift / ProviderManager.swift
    StatusBarController.swift   menu bar
    MenuInfoCardView.swift      dropdown info card (high-contrast, plan/model)
    SettingsView.swift / SettingsWindowController.swift
    SupportView.swift           Support tab (GitHub Sponsors + KakaoPay QR)
    AnalyticsView.swift / AnalyticsWindowController.swift   analytics dashboard
    DockPresence.swift          Dock icon while a window is open
    LineDragOverlay.swift       on-screen drag adjustment for partition lines
    HistoryStore.swift / MiniChartView.swift / NotificationManager.swift
    RefreshScheduler.swift
    LoginWindowController.swift / PopupWebView.swift / HoverInfoController.swift
  .github/FUNDING.yml           repo sponsor button
  PROGRESS.md                   development log
```

## ⚠️ Notes

- Relies on unofficial endpoints/DOM of claude.ai / gemini / chatgpt.com — queries may break if those services change.
- ChatGPT's regular **chat** quota is not exposed anywhere, so it can't be monitored — the **Codex (agent) usage** of the same plan is supported instead.
- Gemini querying is still experimental.

## 📄 License

[MIT](LICENSE)
