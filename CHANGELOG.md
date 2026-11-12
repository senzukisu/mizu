# Changelog

All notable changes to Mizu are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [0.1.3] — 2026-11-12

### Changed
- **Vertical tabs use one top row:** back/forward/reload, the address bar, the menu and the window buttons, like other
  browsers with side tabs. The toolbar is a little lower in both layouts.
- **About Mizu** opens with an animated "current": violet streamlines flowing around a chrome drop.

### Fixed
- With translucency on, a strip of the desktop showed above the toolbar.
- Clicking ⋮ (or any panel button) while its menu is open now closes it instead of reopening it.
- Extensions installed from the Chrome Web Store keep their store ID, so their settings survive reinstalls and updates.

## [0.1.2] — 2026-11-11

### Fixed
- Tabs no longer twitch when a site keeps turning its sound on and off. The sound icon stays for a moment after the
  sound stops and slides in and out instead of popping.

### Added
- **Unload tab** and **Unload other tabs** in the tab context menu. These close the page to free memory; the tab stays
  and reloads when opened. Tabs playing sound are skipped.
- A user guide with a troubleshooting table: [docs/GUIDE.md](docs/GUIDE.md) and [docs/ru/GUIDE.md](docs/ru/GUIDE.md).

## [0.1.1] — 2026-11-11

### Fixed
- **Mizu no longer freezes with many tabs.** Web views are now hosted visually (DirectComposition):
  - there are no engine windows inside Mizu's window any more, so a busy engine can't block the interface;
  - opening, closing and switching tabs stays smooth while pages load.
- A hidden window could loop on focus at startup and stall Explorer.
- The tab strip no longer shakes near its minimum width. With many tabs it scrolls sideways (wheel or touchpad) and fades at the edges.
- The picture-in-picture button now appears on YouTube and other sites that use Trusted Types.
- Changing the default page zoom now applies to open tabs.
- Address bar suggestions span the full width of the address bar.

### Added
- **Customisable keyboard shortcuts** in Settings → Shortcuts. Each action can be re-recorded or reset, and conflicts are resolved automatically.
- **Search suggestions** from the selected search engine (DuckDuckGo, Google, Bing, Brave, Ecosia, Yandex, Wikipedia). There is a separate switch, and they are never used in private windows.
- Picture in picture:
  - previous/next video and 10-second skip buttons in the floating window;
  - settings for the button position and an icon-only button.
- Settings → General → Updates: "Check for updates" and "Update" buttons. Mizu asks before installing a new version it finds at startup.
- A bookmarks bar context menu (open in a new tab, window or private window; copy the link; delete). The bar appears only when it has bookmarks.
- "Open password manager" in Settings → Security, and a bigger close confirmation.
- Installer and uninstaller in Mizu's style, with a choice of install folder.

## [0.1.0] — 2026-11-10

The first public release.

### Install & update
- `MizuSetup` installs Mizu for the current user without administrator rights:
  - a setup window in Mizu's style, where you choose the install folder (default `%LOCALAPPDATA%\Programs\Mizu`);
  - Start menu and optional desktop shortcuts, and registration as a browser for "Default apps";
  - an uninstaller in the same style that can also delete your data. It removes only the files setup installed.
- A portable zip is available as an alternative.
- Mizu updates itself:
  - it checks GitHub 90 seconds after start, then every 6 hours;
  - the release manifest is signed with ed25519 and the installer's SHA-256 is verified before "Restart to update" is offered;
  - the update installs silently and Mizu restarts.
- Windows opens Mizu maximized by default.

### Engine
- Microsoft Edge WebView2 (Chromium) with the Chromium sandbox and site isolation; Windows updates the runtime.
- One browser process tree per profile, and a separate in-memory profile for private windows.
- The internal `mizu://` scheme is served from assets compiled into the binary, with a strict Content Security Policy.

### Interface
- A custom frameless window with the tab strip in the title bar, Windows 11 Mica, and system resize borders.
- **Vertical tabs:**
  - a sidebar you can resize by dragging its edge (double-click resets it) and collapse to icons;
  - the "New tab" button sits under the last tab.
- Tabs:
  - create, close, drag to reorder, pin, duplicate, mute, and reopen closed tabs (the last 25 are kept);
  - a context menu, keyboard shortcuts, and smooth FLIP animations;
  - new tabs open instantly from a pre-created view.
- Omnibox:
  - recognises URLs or searches, suggests from history, bookmarks and internal pages, and supports `@keyword` search engines;
  - shows the security indicator, Shield counter, password key, bookmark star and zoom.
- New tab page: bookmark shortcuts with a "+" tile, and a search engine switcher.
- Themes: light, dark or system, five palettes, a custom accent colour, density, a soft or crisp style, and an animations switch.
- Flicker-free internal pages: the theme is applied before the first paint.
- **Russian translation** of the whole interface. The language follows Windows or is set in Settings.
- Internal pages: New Tab, Settings, History, Bookmarks, Downloads, Passwords, Mizu Shield, Scripts & extensions, Media diagnostics, About, and a blocked-page interstitial.

### Privacy & security
- **Mizu Shield** blocks content with adblock-rust:
  - uBlock Origin / Adblock Plus syntax, cosmetic filtering and scriptlets;
  - default lists: uBlock filters, Privacy, Unbreak, Badware, EasyList, EasyPrivacy and Peter Lowe's list;
  - optional lists: Annoyances and EasyList Cookie;
  - custom lists and rules, a per-site allowlist, and document-level blocking with an interstitial;
  - lists are downloaded with mirrors and retries.
- **Mizu Passwords:**
  - logins are encrypted with Windows DPAPI;
  - a styled "Save password?" prompt appears after sign-in, and logins are filled from the key icon in the address bar;
  - a management page with search, reveal, copy and edit;
  - CSV import and export for Chrome, Edge, Firefox and Bitwarden formats, plus a "never save for this site" list.
- Engine tracking prevention levels, and Global Privacy Control (the `Sec-GPC` header and `navigator.globalPrivacyControl`).
- Site permissions with prompts and remembered decisions; private windows never keep them.
- Clearing browsing data by category and time range, plus clear-on-exit options.
- Pop-ups without a user gesture are blocked, and dangerous external protocol handlers are refused.
- No telemetry.

### Media
- Built-in **picture in picture**: a hover button on videos and the Alt+Shift+P shortcut.
- `mizu://media` reports codecs, system decoders and hardware acceleration.

### Data
- A SQLite profile database with migrations, a WAL journal, automatic backup, and recovery from corruption.
- Session save and restore (periodic and on exit), crash detection with a restore offer, and lazy loading of restored tabs.
- History, bookmarks (folders, bar, drag and drop), download records and site permissions.
- Settings are stored in human-readable TOML with validation; corrupt files are moved aside.

### Downloads
- A download manager that replaces the engine's UI:
  - progress, speed, time left, pause and resume, cancel, retry, open, and show in folder;
  - a configurable folder and an "ask where to save" option.

### Userscripts & extensions
- **Mizu Scripts**, a Greasemonkey/Tampermonkey-compatible userscript manager:
  - `@match`, `@include`, `@exclude`, `@require`, `@run-at`, and the `GM_*` / `GM.*` API, with per-script storage protected by a per-page token;
  - installs from Greasy Fork, Sleazy Fork, OpenUserJS, GitHub (with a jsDelivr mirror), Gist, GitLab, Codeberg and Bitbucket links;
  - installs several links or files at once, with a preview to choose which scripts to add;
  - automatic daily updates through `@updateURL` and `@downloadURL`;
  - per-script site rules: run only on the sites you choose, or never on some sites.
- Chromium MV3 extensions:
  - one-click "Add to Mizu" on Chrome Web Store pages, or paste a store link;
  - install from a `.crx`, `.zip` or unpacked folder;
  - enable, disable, remove, and open options.

### Performance
- Startup in under half a second on a typical PC; the window appears only once the interface is ready.
- Profile-wide engine settings are applied once, views are closed in the background, and low-memory mode applies only after 2 minutes idle.
- The browser UI renders at most once per frame; download progress is pushed about 4 times a second; the bookmarks bar is cached.
- Sleeping tabs (configurable delay), efficiency mode, and a hardware acceleration switch.

### Known limitations
- Restored tabs keep their URL but not their back/forward history, because the engine does not expose it.
- `GM_xmlhttpRequest` is subject to CORS, and `GM_registerMenuCommand` has no menu yet.
- Generic (class/id-based) cosmetic filters are not applied; site-specific ones are.
- Extensions have no toolbar buttons, so their options and popups open in a tab. Private windows run without extensions.
- Installing from Microsoft Edge Add-ons is not reliable yet; the Chrome Web Store works.
- Windows only.
