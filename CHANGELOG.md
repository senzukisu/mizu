# Changelog

All notable changes to Mizu are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [0.3.0] — 2026-11-23 · "Currents"

### Added
- **Welcome to Mizu.** A short tour in its own window on first start and once after this update: what Mizu does,
  theme / tab layout / accent colour, default browser, and importing from another browser. Every step can be
  skipped and says where to find it later; the language can be switched on the first screen. The browser window
  opens when the tour is finished or closed.
- **Import from other browsers** (`mizu://import/`, Settings → General, the bookmarks manager): Chrome, Edge,
  Yandex Browser, Brave, Vivaldi, Chromium, Opera, Opera GX and Firefox profiles on this PC. Bookmarks (the source's
  bar onto Mizu's bar without duplicates, the rest into an "Imported from …" folder), up to 50 000 recent history
  entries, and search engines from Chromium browsers. The other browser can stay open and is never changed.
- **Tab groups** in both layouts: Add to new group / Add to … / Remove from group in the tab menu; click the chip to
  collapse or expand, right-click it to rename, recolour, ungroup or close its tabs. Dragging a tab into or out of a
  group works too, and groups are kept with the session.
- **Quick actions** in the address bar: type `>` for clear this site's cookies and data, mute or unmute all tabs,
  unload or close other tabs, new group, private window, bookmarks bar, find, and Mizu's pages. Russian words work.
- **Unit conversion** in the address bar: length, mass, volume, speed, data size and temperature
  (`10 km in mi`, `70 f to c`, `5 ft в см`); Enter copies the answer.
- Settings → General: default browser status with a button, and "Show the welcome tour".

### Changed
- The About animation draws in a background worker at 60 fps, so the page reacts instantly while it plays.

### Fixed
- A closed tab could stay on screen for a few seconds while its fade-out waited for the window.
- Clicking a group chip twice quickly opened its menu instead of just collapsing and expanding.

## [0.2.2] — 2026-11-22

### Added
- **Music and video hub.** A music button appears in the toolbar while a tab plays (its bars move while something
  is playing). Its panel lists every tab with sound or video, with the track, artist and cover art the site
  publishes, a progress bar, previous / play-pause / next, a volume slider and "Go to tab".
  - Previous/next use the site's own controls (YouTube, YouTube Music, Spotify, SoundCloud, VK and others), as the
    keyboard's media keys would.
  - **Volume per site**, remembered and applied every time the site plays; listed in Settings → Media.
  - **One sound at a time** (optional): when a tab starts playing, the others pause.
  - The keyboard's **media keys** work while Mizu is focused, and there are shortcuts: Ctrl+Alt+Space (play/pause),
    Ctrl+Alt+Right / Left (next / previous track).
- **New tab widgets:** "Now playing" with controls, "Recently closed" and a notepad kept on this PC. Tile size
  (small, medium, large) and your name in the greeting.
- **Interface font:** Mizu's standard, the Windows system font, or any font installed on the PC, with a preview.
- **Toolbar:** choose whether to show the bookmark star, the Shield button, the music button and the Home button, and
  whether the downloads button is always there.
- A third control style, **Round**, and an option to **tint the window with the accent colour**.

### Fixed
- **Doubled window buttons:** Windows drew its own minimise/maximise/close buttons under Mizu's when the window was
  translucent.
- **Pages that load very fast** (from the cache or the local network) missed Mizu's page scripts: cosmetic filtering,
  Global Privacy Control, userscripts and password saving. The scripts now run even when the page is quicker than
  their registration.
- Duplicate entries in the Russian dictionary; a few labels used a different translation than intended.

### Development
- **CI** on every push: interface script checks, Clippy with warnings as errors, and the test suite.
- `scripts/check-ui.mjs`: syntax of every interface script, duplicate translations, and global name clashes between
  shared and page scripts (the cause of the blank Settings page in 0.2.0 development).

## [0.2.1] — 2026-11-16

### Added
- **Four new palettes:** Fuji (lavender), Umi (sea teal), Kohaku (amber) and Yoru (deep night, near-black in dark
  mode), each in light and dark.
- **More accent colours** (17 plus your own), with a check mark on the chosen one, and a **live preview** of the browser
  in Settings → Appearance that follows every change.
- **New tab backgrounds:** Accent (follows your accent colour), Forest, Cherry blossom, Ember, Nebula, Glacier and
  Graphite, 13 in all.
- **Your own picture** as the new tab background. It is resized and kept only on this PC; a slider darkens it so the
  clock and tiles stay readable.
- The new tab Customize panel can hide the shortcut tiles and links to the colour and theme settings.

## [0.2.0] — 2026-11-15 · "Clear Water"

### Added
- **Cookies you can trust:**
  - with "Continue where I left off", session cookies are kept (encrypted with your Windows account), so sites stay
    signed in after a restart;
  - **third-party cookies are blocked** by default (Settings → Privacy); embedded ads and trackers can no longer set or
    read cookies;
  - the lock icon shows a site's cookies, and **Clear site data** removes its cookies and storage, then reloads it.
- **Links open apps:** `steam:`, `tg:`, `zoom:`, `mailto:` and other app links work. Mizu asks first and can always
  allow a site; allowed sites are listed in Settings → Privacy. Dangerous schemes stay blocked.
- **Find on page** (Ctrl+F) in Mizu's own bar: match counter, previous/next (Enter, Shift+Enter, F3), match case, Esc
  to close.
- **Pinned tabs in the sidebar:** a grid of icons above the list. Drag a tab up to pin it and down to unpin it. A
  pinned tab remembers its page: after you browse elsewhere its icon gets a dashed outline, and the tab menu offers
  "Back to pinned page" or "Pin this page instead".
- **Address bar:** a calculator (`2+2*3`, `15% * 80`; Enter copies the answer) and "Switch to this tab" for tabs that
  are already open, including in other windows.
- **New tab page:** drag tiles to reorder them, and a **Customize** panel with backgrounds (Wave, Dawn, Dusk, Aurora,
  Sea, None) and switches for the clock and the Shield counter.
- **Settings:** search across every section (Ctrl+F or /), and a **Restart Mizu** button for changes that need a
  restart (tabs come back).
- **Downloads:** coloured file-type badges, grouping by day, a summary of active downloads, and buttons that open the
  downloads folder.
- **About Mizu:** feature tiles that open their section, a "What's new" card (`mizu://about/#new`) and a link to all
  releases.

### Changed
- **Bookmarks bar, Other bookmarks and new tab shortcuts are three separate lists.** Ctrl+D saves to Other bookmarks
  (or the folder you last chose), the bar shows only what you put there, and the new tab tiles have their own list.
  Your existing tiles were copied once, so nothing disappeared.
- The bookmarks bar no longer appears just to show "Other bookmarks".
- Settings cards have calmer headings; the language setting is a single row.

### Fixed
- **The address bar updates immediately when you switch tabs**, in the same frame as the tab.
- Cookies and site data are flushed properly when Mizu closes.
- `mailto:` and other app links were refused instead of opening the app.

## [0.1.4] — 2026-11-12

### Improved
- **About Mizu scrolls smoothly.** The animated hero computes its streamlines once per resize and draws them in a
  single pass. It animates at 30 frames per second, pauses while you scroll and stops when it is off screen.
  Rendering cost on that page dropped by about 70% (page process about 15× less).
- Code clean-up: unused interface leftovers removed, and Clippy reports no warnings.

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
- Per-script site rules in Mizu Scripts: run only on the sites you choose, or never on some sites.

## [0.1.0] — 2026-11-10

The first public release.

### Install & update
- `MizuSetup` installs Mizu for the current user into `%LOCALAPPDATA%\Programs\Mizu` without administrator rights:
  - Start menu and optional desktop shortcuts, and registration as a browser for "Default apps";
  - an uninstaller that can also delete your data.
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
  - automatic daily updates through `@updateURL` and `@downloadURL`.
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
