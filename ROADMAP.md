# Mizu roadmap

Where Mizu is going after the 0.1 series. Versions are ordered by what matters most in everyday use, then by
technical dependencies. There are no dates: a version ships when its "done when" checks pass.

**Principles:** stability → security → performance → maintainability → extensibility. A new feature never makes
the first three worse.

Ideas and requests are welcome in [issues](https://github.com/senzukisu/mizu/issues).

## At a glance

| Version | Name | Focus | Status |
|---|---|---|---|
| 0.1.x | — | First releases: Shield, passwords, scripts, vertical tabs, visual hosting, auto-updates | ✅ Released |
| **0.2** | Clear Water | Fixes and polish, cookies, opening links in apps, faster loading | 🔜 Next |
| 0.3 | Currents | Spaces, tab groups, focus mode, quick actions | Planned |
| 0.4 | Deep Water | Privacy and data: fingerprinting defence, backups | Planned |
| 0.5 | Tide | Passwords v2 and optional end-to-end encrypted sync | Planned |
| 0.6 | Estuary | Built-in tools: PDF, translation, media hub | Planned |
| 0.7 | Rivers | Linux, macOS, Android and iOS | Planned |
| 1.0 | Ocean | Stable: accessibility, guarantees, long-term engine plan | Planned |

## Already in 0.1.x

- **Install and update:** signed automatic updates; installer and uninstaller with a choice of folder.
- **Speed:** visual hosting, so Mizu stays responsive with many tabs; sleeping and unloadable tabs.
- **Tabs:** vertical tabs with a one-row toolbar.
- **Features:**
  - Mizu Shield;
  - the password manager;
  - userscripts with site rules;
  - Chrome extensions;
  - customisable shortcuts;
  - search suggestions;
  - picture in picture with video switching.
- **Language:** English and Russian interface and documentation.

---

## 0.2 — "Clear Water": polish and trust

**Goal:** make Mizu comfortable as the default browser. Fix what gets in the way every day, and add what people
miss most.

**Fixes**
- **The address bar lags when switching tabs.** It must show the new tab's address, security icon and buttons
  immediately, in the same frame as the tab switch.
- **Page loading feels slow.** The plan:
  - start navigation without waiting for the UI;
  - preconnect to the address being typed;
  - show a clearer progress bar;
  - restore background tabs gradually instead of all at once.
- Text and wording across the interface and both languages: consistent terms, shorter descriptions, no leftover
  English in the Russian UI.

**Features**
- **Cookies:**
  - block third-party cookies everywhere or per site;
  - a cookie list per site, with delete;
  - "keep me signed in" exceptions;
  - optional automatic cleanup of sites you haven't visited in N days.
- **Open links in apps.**
  - Links like `steam://`, `zoom://`, `tg://` and `mailto:` open in their program, with a clear prompt and an
    "always allow for this site" option. Allowed apps can be managed in Settings.
  - "Open with…" for downloaded files and for media links, for example a video in a player.
- **Pinned tabs in the sidebar:**
  - pinned tabs as a compact icon grid at the top of the sidebar;
  - drag to pin and unpin;
  - pinned tabs keep their site when you navigate away (one click returns to the pinned address).
- **Address bar:**
  - richer suggestions (open tabs, "switch to tab", calculator);
  - an edit-and-copy friendly URL view;
  - clearer site information.
- **New tab page:** rearrange and rename tiles, folders on the page, optional wallpaper, and a quick-settings
  button.
- **About Mizu:** release notes for the installed version, links to the guide and changelog, and a lighter
  animation for slower PCs.
- **Find bar** in Mizu's style (Ctrl+F) instead of the engine's.
- **Import from other browsers** (Chrome, Edge, Firefox, Opera, Brave, Vivaldi): bookmarks, history and search
  engines, with a preview.
  - Passwords come only from the browsers' own CSV export; Mizu never decrypts other browsers' stores.
- **Shield:** generic and procedural cosmetic filters, plus a per-site page that gathers zoom, permissions, Shield
  and cookies.
- **Accessibility:** screen reader support for visually hosted pages (Mizu's own UI Automation provider).

**Technical work**
- Integration tests that drive the real UI, and CI (tests, Clippy, UI script checks).
- Updater hardening: roll back if a new version fails to start, plus a beta channel.
- A code-signed installer (Authenticode).
- Local crash reports, which are never uploaded automatically.

**Done when:**
- switching tabs updates the address bar in the same frame;
- a cold start to the first page is under 400 ms on a mid-range laptop;
- the updater survives forced failures (bad signature, interrupted download, failed start);
- a week of daily use brings no crash reports.

## 0.3 — "Currents": spaces and focus

**Goal:** keep many tasks apart without losing tabs.

- **Spaces:** named tab sets (Work, Study, Play) with their own colour and icon. Each space restores separately.
- **Tab groups** in both layouts, and **tree tabs** in the sidebar (tabs opened from a page sit under it).
- **Focus mode:** hides the interface until you hover, and holds back site notifications.
- **Quick actions** in the address bar: `>` commands ("clear cookies for this site", "mute all tabs"), calculator
  and unit conversion. Everything is computed locally.
- **Split view:** two tabs side by side.

**Technical:** a tree tab model and session format v2 with a tested migration.

**Done when:** a 300-tab session restores in under a second, and the migration is verified on real profiles.

## 0.4 — "Deep Water": privacy and data

**Goal:** stronger protection with no breakage.

- **Shield dashboard v2:** a per-site breakdown, a timeline of blocked requests, and a fingerprinting report.
- **Fingerprinting defence:** protective values for high-entropy APIs (canvas and audio noise, bucketed hardware
  details). Mizu won't claim anonymity it can't provide.
- **HTTPS-First mode**, with a warning when a site works only over HTTP.
- **Encrypted local backup** of the profile with a passphrase (Argon2id + XChaCha20-Poly1305, audited crates).

**Done when:** popular sites work with fingerprinting defence on, and a backup restores bit for bit.

## 0.5 — "Tide": passwords and sync

**Goal:** your data on all your devices, readable only by you.

- **Passwords v2:**
  - Windows Hello before showing or filling a password;
  - a password generator;
  - a breach check with k-anonymity (only hash prefixes leave the device);
  - passkeys where the engine supports them.
- **Mizu Sync (optional):** end-to-end encrypted sync of bookmarks, history, tabs, settings and passwords. The
  server stores only ciphertext, and you can host it yourself.

**Done when:** an independent review of the password and sync design has no open high-severity findings.

## 0.6 — "Estuary": built-in tools

- **PDF viewer** with Mizu's toolbar: annotations, forms and printing.
- **Translation** with a local model where the hardware allows. A cloud provider is used only with explicit consent.
- **Media hub:** every tab playing sound or video in one panel, with Windows media keys.
- **Reading mode** with Mizu's typography.
- **Extension buttons** and popups on the toolbar, once the engine exposes them.

## 0.7 — "Rivers": Linux, macOS and mobile

WebView2 exists only on Windows, so each platform gets its own engine behind the `mizu-engine` interface:

| Platform | Engine | Notes |
|---|---|---|
| macOS | WKWebView | H.264/AAC from the system; Shield through WebKit content rules plus Mizu's cosmetic and scriptlet layer |
| Linux | WebKitGTK (or CEF) | WebKitGTK ships with distributions; CEF only if extensions are needed there |
| Android | Android System WebView | A mobile layout around the same HTML interface |
| iOS / iPadOS | WKWebView | Apple requires WebKit on iOS (outside the EU) |

- **Already portable:** the interface (`assets/ui`), storage, settings, Shield's filter engine, userscripts and the
  tab model. Passwords switch to each platform's keychain instead of DPAPI.
- **Platform specific:** the engine adapter, windows and input, the installer and updates.
- **Mobile layout:** a bottom address bar, a tab grid and gestures, sharing settings and features with desktop.
- **Extensions** stay Windows-first. On WebKit platforms, userscripts and Shield cover the most common needs.

**Done when:** the core features work on every platform, as tracked in a parity table in
[ARCHITECTURE](docs/ARCHITECTURE.md).

## 1.0 — "Ocean": stable

- At least 99.8% of sessions without a crash (opt-in local statistics), and no known data-loss bugs.
- An accessibility audit passed: keyboard only, screen readers, high contrast.
- Documented and versioned compatibility for extensions and userscripts.
- A long-term engine strategy (WebView2, CEF or Servo) with a migration plan.

---

## Ideas we're weighing

| Idea | Decision |
|---|---|
| Servo engine | Watching. A promising Rust engine, but not ready for everyday sites yet. We check again at every release. |
| AI features | Only optional and local first (summarise, translate). Never on by default, and page content never leaves the device silently. |
| More interface languages | Yes. A translation is a single dictionary file, and contributions are welcome. |
| Firefox add-ons | Not on WebView2: it runs only Chromium extensions. To be revisited if a future engine supports them. |
| Built-in crypto wallet | No. It's outside a browser's job and adds attack surface. |
