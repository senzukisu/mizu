# Changelog

All notable changes to Mizu are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-11-10

First development release.

### Engine
- Microsoft Edge WebView2 (Chromium) engine with Chromium sandbox and site isolation; the runtime is updated by Windows.
- One browser process tree per profile; separate InPrivate profile for private windows.
- Internal `mizu://` scheme served from assets compiled into the binary, with a strict Content Security Policy.

### Interface
- Custom frameless window with a tab strip in the title bar, Windows 11 Mica, system resize borders.
- Tabs: create, close, drag to reorder, pin, duplicate, mute, reopen closed (25 kept), context menu, keyboard shortcuts.
- Omnibox: URL or search detection, local suggestions (history, bookmarks, internal pages), `@keyword` search engines,
  security indicator, Shield counter, bookmark star, zoom indicator.
- Flyouts: main menu, tab menu, downloads panel, Shield panel, site information, bookmark editor, permission prompts,
  close confirmation.
- Themes: light/dark/system, five palettes, custom accent colour, density, soft/crisp style, animations switch.
- Internal pages: New Tab, Settings, History, Bookmarks, Downloads, Mizu Shield, Scripts & extensions, Media diagnostics,
  About, blocked-page interstitial.

### Privacy & security
- Mizu Shield content blocking on adblock-rust: uBlock Origin / Adblock Plus syntax, cosmetic filtering and scriptlets,
  default subscriptions (uBlock filters, Privacy, Unbreak, Badware, EasyList, EasyPrivacy, Peter Lowe), custom lists and rules,
  per-site allowlist, document-level blocking with interstitial.
- Engine tracking prevention levels, Global Privacy Control (`Sec-GPC` header and `navigator.globalPrivacyControl`).
- Site permissions with prompts and remembered decisions; private windows never persist them.
- Clear browsing data by category and time range; clear-on-exit options.
- Pop-ups without a user gesture are blocked; dangerous external protocol handlers are refused.
- No telemetry.

### Data
- SQLite profile database with migrations, WAL journal, automatic backup and recovery from corruption.
- Session save/restore (periodic and on exit), crash detection and restore offer, lazy loading of restored tabs.
- History, bookmarks (folders, bar, drag-and-drop), download records, site permissions.
- Settings in human-readable TOML with validation; corrupt files are moved aside.

### Downloads
- Download manager replacing the engine's UI: progress, speed, time left, pause/resume, cancel, retry, open, show in folder,
  configurable folder, "ask where to save".

### Userscripts & extensions
- Mizu Scripts: Greasemonkey/Tampermonkey-compatible userscript manager with install-from-link, `@require`, `GM_*` API
  and per-script storage protected by a per-page token.
- Chromium MV3 extensions: one-click "Add to Mizu" on Chrome Web Store / Edge Add-ons pages (or paste a store link),
  or install from a `.crx`, `.zip` or unpacked folder; enable, disable, remove, open options.

### Performance
- Chrome UI renders at most once per frame; download progress pushed ~4×/s; bookmarks bar cached.
- Sleeping tabs (configurable delay), efficiency mode (low memory target for background tabs), hardware acceleration switch.

### Known limitations
- Restored tabs keep their URL but not their back/forward history (the engine does not expose it).
- No built-in password manager or sync yet; engine password saving is off by default.
- `GM_xmlhttpRequest` is subject to CORS; `GM_registerMenuCommand` has no menu yet; regex `@include` rules support a common
  subset of syntax.
- Generic (class/id based) cosmetic filters are not applied; site-specific ones are.
- Extensions have no toolbar buttons; options/popups open in a tab. Private windows run without extensions.
- No automatic updater for Mizu itself yet (the engine is updated by Windows).
- Windows only.
