# Mizu roadmap

Where Mizu is going after the 0.1 series. Versions are ordered by what matters most in everyday use, then by
technical dependencies. There are no dates: a version ships when its "done when" checks pass.

**Principles:** stability → security → performance → maintainability → extensibility. A new feature never makes
the first three worse.

**Where we are:** 0.4.0 is out — the first step of "Deep Water". Next: fingerprinting defence (0.4.1), then
encrypted backups; integration tests and a code-signed installer are still open from 0.3.x.
What already shipped is listed version by version in the [changelog](CHANGELOG.md).

Ideas and requests are welcome in [issues](https://github.com/senzukisu/mizu/issues).

## At a glance

| Version | Name | Focus | Status |
|---|---|---|---|
| 0.1.x | — | First releases: Shield, passwords, scripts, vertical tabs, visual hosting, auto-updates | ✅ Released |
| 0.2 | Clear Water | Cookies, links to apps, find bar, pinned sidebar tabs, polish | ✅ Released |
| 0.3 | Currents | Import, welcome tour, tab groups, quick actions | ✅ Released |
| 0.3.1 | Currents | Site settings, cookie cleanup, Open with, crash reports | ✅ Released |
| 0.3.2 | Currents | Spaces, split view | ✅ Released |
| 0.3.3 | Currents | Tree tabs, new tab folders, per-site zoom | ✅ Released |
| 0.3.4 | Currents | Generic and procedural cosmetic filters, open in media player | ✅ Released |
| 0.3.5 | Currents | Faster loading, updater rollback and beta, new settings, experimental features | ✅ Released |
| 0.3.6 | Currents | What's new after updates, sites follow the theme, restart fix | ✅ Released |
| **0.3.x** | Currents | Integration tests, code-signed installer | 🔜 Next |
| 0.4.0 | Deep Water | HTTPS-First, Shield dashboard v2 | ✅ Released |
| **0.4.x** | Deep Water | Fingerprinting defence, encrypted backups | 🔜 Next |
| 0.5 | Tide | Passwords v2 and optional end-to-end encrypted sync | Planned |
| 0.6 | Estuary | Built-in tools: PDF, translation, reading mode; screen readers | Planned |
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

## 0.2 — "Clear Water" ✅

Released 2026-11-15. What it brought (details in the [changelog](CHANGELOG.md)):

- **Cookies:** session cookies survive a restart, third-party cookies blocked by default, a site's cookies in the
  lock icon with "Clear site data".
- **Links to apps** (`steam:`, `tg:`, `zoom:`, `mailto:`…) with a prompt, "always allow" per site and a list in
  Settings.
- **Find on page** in Mizu's own bar.
- **Pinned tabs in the sidebar** as an icon grid with drag to pin; pinned tabs remember their page.
- **Address bar:** calculator, "switch to tab"; the address updates in the same frame as a tab switch.
- **New tab:** its own shortcuts (separate from the bookmarks bar), drag to reorder, backgrounds, Customize panel.
- **Settings search**, a restart button, calmer cards; **downloads** grouped by day with file-type badges; a renewed
  **About Mizu** with "What's new".

Moved to 0.3: import from other browsers, Shield cosmetic filters and the per-site page, the UI Automation provider,
loading improvements, "Open with…", automatic cookie cleanup, CI, updater rollback and the beta channel, crash reports.

## 0.3 — "Currents" ✅

Released 2026-11-23 (details in the [changelog](CHANGELOG.md)):

- **Move in:** import bookmarks, history and search engines from Chrome, Edge, Brave, Vivaldi, Opera
  and Firefox, with a preview of what each profile holds.
- **Welcome to Mizu:** a one-time tour in its own window: style, default browser, import.
- **Tab groups** with names and colours in both layouts, kept with the session.
- **Address bar:** `>` quick actions and unit conversion.
- Since 0.2.2: CI on every push and interface checks.

## 0.3.x — "Currents", continued

**0.3.1 ✅** (2026-11-24): site settings page (Shield, cookies, "keep me signed in", forget the site, permissions),
automatic cookie cleanup, "Open with…" for downloads, local crash reports.

**0.3.2 ✅** (2026-11-24): spaces (separate tab sets with a name, colour and icon) and split view; importing no
longer fills Mizu's bookmarks bar.

**0.3.3 ✅** (2026-11-24): tree tabs in the sidebar, folders of new tab shortcuts, per-site zoom.

**0.3.4 ✅** (2026-11-24): generic and procedural cosmetic filters in Shield; "Open in media player" for videos and
media links.

**0.3.5 ✅** (2026-11-25): connect while typing, a loading bar that follows real progress, pinned tabs loaded in the
background; updates roll back if the new version fails to start, and a beta channel; settings reorganised; an
Experimental page. Measured: a 300-tab session with groups opens its window in ~0.35 s.

Still to come, roughly in this order:

**Technical work**
- Integration tests that drive the real interface.
- A code-signed installer (Authenticode).

**Done when:**
- ~~a 300-tab session with groups restores in under a second~~ — done in 0.3.5 (~0.35 s);
- ~~the updater survives forced failures (bad signature, interrupted download, failed start)~~ — done in 0.3.5;
- the interface is covered by integration tests and the installer is signed.

## 0.4 — "Deep Water": privacy and data

**Goal:** stronger protection with no breakage, and your data safe even if this PC is not.

Shipped in steps, each one usable on its own:

| Step | What you get | How it works |
|---|---|---|
| **0.4.0 ✅** | **HTTPS-First** · Shield dashboard v2 | Top-level `http://` navigations are retried as `https://` (on failure Mizu shows a clear "this site has no secure version — continue?" page, remembered per site). The Shield page gains a per-site breakdown and a timeline from the counters Mizu already keeps. |
| **0.4.1** | **Fingerprinting defence** (standard) | A document-start script gives high-entropy APIs protective values: small per-site, per-session noise for canvas readback and audio, bucketed `hardwareConcurrency`/`deviceMemory`, no battery API, a common font list. Off per site from the lock icon. |
| **0.4.2** | Fingerprinting report · strict mode | Shield shows which scripts read which APIs on the page. Strict mode also hides WebGL details and screen size precision. |
| **0.4.3** | **Encrypted backup and restore** | One file with bookmarks, history, passwords, settings, scripts and tiles; key from a passphrase with Argon2id, sealed with XChaCha20-Poly1305 (RustCrypto crates, no own crypto). Restore checks integrity first and never half-applies. Optional scheduled backup to a folder you pick. |

**Ground rules**

- Every protection has a per-site exception in Site settings, and a short "what this may break" note.
- Nothing claims anonymity: the guide says plainly what fingerprinting defence can and cannot do.
- New settings land on existing pages (Mizu Shield, Privacy, Passwords & security); nothing new is on by default until
  it has passed the site list below.

**Test list for "no breakage":** Google and Microsoft sign-in, YouTube, Twitch, Discord, GitHub, Figma, Notion,
two banks, two shops with 3-D Secure, Cloudflare and hCaptcha checks.

**Done when:**
- the test list works with HTTPS-First and standard fingerprinting defence on;
- the noise is stable within a site and a session, and different across sites (checked by a test page);
- a backup restores bit for bit on a clean profile, and a wrong passphrase or a damaged file is refused with a clear
  message.

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
- **Reading mode** with Mizu's typography.
- **Extension buttons** and popups on the toolbar, once the engine exposes them.
- **Screen readers** (moved here from 0.3.x): Mizu's own UI Automation provider so NVDA and Narrator read visually
  hosted pages and the interface, plus full keyboard navigation of the sidebar and panels. Done when NVDA and
  Narrator read pages, menus and panels.

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
