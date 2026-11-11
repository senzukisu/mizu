# Mizu Roadmap

This is Onibi Lab's plan for Mizu after 0.1.0. It is ordered by value to everyday users and by technical dependency.
Every item is something the chosen architecture can actually deliver. Anything that depends on a future engine is
marked as such. Dates are deliberately absent: a release ships when its exit criteria are met.

**Guiding principles:** stability → security → performance → maintainability → extensibility.
New features never come at the cost of the first three.

---

## 0.2.0 — "Clear Water": polish and trust

The goal is to make 0.1 something people keep as their default browser.

Already in 0.1.0: a signed auto-updater, a per-user installer, vertical tabs, a password manager with CSV
import/export, picture in picture, and a Russian translation.

**Features**
- Updater hardening: roll back if the new version fails to start, delta updates, and a "beta" channel.
- Code-signed installer (Authenticode) and an MSIX package for the Microsoft Store.
- Find bar integrated into the Mizu UI, using the engine's Find API.
- Bookmark folders on the bookmarks bar open as flyout menus; drag a page's padlock icon onto the bar.
- Generic cosmetic filtering (class/id selectors via a lightweight DOM observer), procedural cosmetic filters.
- Per-site settings page: zoom, permissions, Shield, cookies.
- HTTPS-First mode: upgrade `http://` navigations, with an interstitial when only HTTP works.

**Technical work**
- Integration tests that drive the real UI (WebView2 DevTools protocol), plus a JS syntax/lint step in CI.
- Crash reporting that stays local (minidumps in the profile, opt-in sharing).
- Startup profiling: target under 400 ms to first paint on a mid-range laptop.

**Depends on:** 0.1.0.
**Done when:** the updater survives forced failure tests (bad signature, interrupted download, failed start), the installer
passes Windows App Certification, and a week of dogfooding produces no crash reports.

## 0.3.0 — "Currents": workspaces and focus

**Features**
- **Mizu Spaces:** named tab sets (Work, Study, Play) with their own colour and icon, switchable from the tab strip.
  Each space is restored independently.
- **Tree tabs** in the vertical sidebar: tabs opened from a parent are grouped under it.
- **Tab groups** in the horizontal strip (collapse, colour, name).
- **Mizu Focus:** a distraction-free mode that hides the toolbar and tab strip (revealed on hover), holds back site
  notifications, and optionally applies a per-space site blocklist.
- **Quick Actions** in the omnibox: `>` commands ("> clear cookies for this site", "> mute all tabs",
  "> open settings privacy"), calculator and unit conversion, all computed locally.

**Technical work:** move the tab model from `TabStrip` to a tree model in `mizu-tabs`; session format v2 with a migration.
**Depends on:** 0.2.0 (updater, so session-format migrations can ship safely).
**Done when:** a 300-tab session restores in under 1 s, and the session v1 → v2 migration is tested on real profiles.

## 0.4.0 — "Deep Water": privacy and data

**Features**
- **Mizu Shield dashboard v2:** per-site breakdown, a timeline of blocked requests, a fingerprinting report.
- **Fingerprinting defence:** standard-but-protective values for high-entropy APIs (canvas/audio readback noise,
  `navigator.hardwareConcurrency` bucketing) through document-start scripts. We will not claim anonymity.
- **Cookie controls:** block third-party cookies globally or per site, and an automatic cleanup of cookies for sites
  you haven't visited in N days.
- **Import from other browsers** (Chrome, Edge, Firefox, Opera, Brave, Vivaldi): bookmarks, history, open tabs, search
  engines, settings. A preview screen lets you pick categories. Passwords and cookies come only through the
  official export formats (CSV/HTML) the user creates in the source browser. Mizu will not decrypt other browsers'
  DPAPI-protected stores.
- **Encrypted local backup** of the profile, with a user passphrase (Argon2id + XChaCha20-Poly1305 from audited crates).

**Depends on:** 0.3.0 (spaces are part of what gets imported and backed up).
**Done when:** imports from each source browser are round-trip tested on fixtures, and backup restore is verified bit-for-bit.

## 0.5.0 — "Tide": passwords and sync

**Features**
- **Passwords v2:** Windows Hello confirmation before revealing or filling a password, a password generator,
  a breach check with k-anonymity (only hash prefixes leave the device), and passkeys where the engine provides them.
- **Mizu Sync (optional):** end-to-end encrypted sync of bookmarks, history, open tabs, settings and passwords
  across devices. The server only stores ciphertext. Self-hostable server published under an OSI license.

**Depends on:** 0.4.0 (backup encryption format is reused for sync).
**Done when:** an independent security review of the password and sync design has no unresolved high-severity findings.

## 0.6.0 — "Estuary": built-in tools

- **PDF viewer** with a Mizu-styled toolbar on top of the engine's renderer: annotations, form filling, printing.
- **Translator:** page translation with a local model where hardware allows (no text sent off-device by default);
  an optional cloud provider only with explicit consent.
- **Mizu Media:** a global media hub flyout listing every tab playing audio or video, with play/pause/mute and
  picture in picture. Integrates with Windows media keys.
- **Developer tools additions:** network-request log export, quick viewport and device emulation presets, request
  blocking rules from DevTools into Mizu Shield.
- Extension toolbar buttons and popups, once the engine exposes action UI (otherwise through a Mizu-hosted popup surface).

## 0.7.0 — "Rivers": cross-platform

- **Linux** (WebKitGTK or CEF) and **macOS** (WKWebView or CEF) builds behind the `mizu-engine` trait. Only the engine
  and windowing layers are platform specific; the UI, storage, privacy, scripts and tabs crates are already portable.
- Decision point: evaluate CEF as a single engine on all platforms (needs codec licensing for H.264/AAC
  in our own builds) against the native engines (which differ in features).

**Done when:** feature parity tables in docs/ARCHITECTURE.md are green for the core feature set on all three platforms.

## 1.0.0 — "Ocean": stable

- Stability bar: crash-free sessions ≥ 99.8% in opt-in local statistics; no known data-loss bugs.
- Accessibility audit (keyboard-only use, screen readers, high contrast) passed.
- Documented, versioned extension and userscript compatibility guarantees.
- Long-term engine strategy committed (WebView2, CEF or Servo) with a migration plan.

---

## Ideas under evaluation

| Idea | Verdict |
|---|---|
| **Servo engine** | Watch closely. It is a promising Rust engine but not yet compatible enough for daily browsing. Revisit every release. |
| **Built-in crypto wallet** | No. It is out of scope and expands the attack surface. |
| **AI assistant** | Only as optional, local-first features (summarize page, translate). Never on by default, never with page content leaving the device silently. |
| **Reading mode** | Yes, around 0.6, as a document-start reader view with Mizu typography. |
| **Split view (two tabs side by side)** | Yes, around 0.3/0.4. The engine supports multiple visible views per window. |
| **More interface languages** | Yes. Translations are a single dictionary file; contributions are welcome. |
| **Mobile** | Not before 1.0. |
