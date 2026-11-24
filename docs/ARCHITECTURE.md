# Mizu architecture

How Mizu is put together, for people changing it. The short version: one Rust process owns the windows and all
decisions; web pages and Mizu's own interface are WebView2 views drawn into those windows; only `mizu-engine` talks
to WebView2.

**Contents:** [Engine choice](#engine-choice) · [Crates](#crates) · [Process and window model](#process-and-window-model) ·
[Event flow](#event-flow) · [UI protocol](#ui-protocol) · [Tab model](#tab-model) ·
[Content blocking](#content-blocking-pipeline) · [Page scripts](#page-scripts) · [Storage](#storage-layout) ·
[Sessions](#session-and-crash-recovery) · [Passwords](#passwords) · [Userscript site rules](#userscript-site-rules) ·
[Translations](#translations) ·
[Updates](#updates) · [Installer](#installer)

## Engine choice

| Option | Compatibility | Codecs | Updates & security | Footprint | Verdict |
|---|---|---|---|---|---|
| **WebView2** (Edge/Chromium) | Current Chromium | **H.264, AAC licensed by Microsoft**; HEVC/AC-3 via system decoders | Evergreen runtime serviced by Windows Update | Runtime shared with the OS (not shipped by us) | **Chosen** |
| CEF | Current Chromium | Standard builds have **no H.264/AAC**; proprietary builds need our own patent licensing | We must rebuild/ship Chromium (~250 MB) on every security release | Large | Planned evaluation for cross-platform (0.7) |
| Servo | Improving, not daily-driver ready | Limited | Young | Small | Watched |
| Tauri/wry | Wraps the same system WebViews | Same as underlying | Same | Small | Too abstracted: no download, permission, profile, request-interception or extension control we need |

WebView2 gives Mizu Chromium's multi-process sandbox and site isolation, evergreen security updates, and licensed
proprietary codecs, while Mizu stays a small native application. Everything engine-specific is isolated in one crate
(`mizu-engine`), so another engine can be added behind the same interface later.

## Crates

```
crates/
├── mizu-core         ids, omnibox classification, profile paths, internal page registry (no deps on Windows)
├── mizu-settings     settings schema, defaults, validation, atomic TOML persistence
├── mizu-storage      SQLite: history, bookmarks, new tab tiles, downloads, permissions, session, logins;
│                     import from other browsers; migrations, backup, recovery
├── mizu-tabs         tab strip model: order, pinning, groups, spaces, tree tabs, closed tabs, snapshots – pure logic
├── mizu-privacy      Mizu Shield: adblock-rust engine, filter list store/updater, permissions catalogue, stats
├── mizu-userscripts  Mizu Scripts: metadata parser, match patterns, script store, GM_* bootstrap
├── mizu-media        media diagnostics (Media Foundation decoders, DXGI GPUs)
├── mizu-ui           UI protocol (typed JSON commands/events) + embedded HTML/CSS/JS assets
├── mizu-engine       the only crate that touches WebView2/COM: Engine, View, Download, events
├── mizu-update       signed release manifest: ed25519 verification, SHA-256, version compare; `mizu-sign` tool
├── mizu-setup        MizuSetup.exe: per-user installer/uninstaller with the browser embedded
└── mizu-browser      the application: windows, tabs, flyouts, passwords, shortcuts, updater, internal page data
assets/
├── ui/               chrome (tab strip/toolbar/sidebar), flyout (menus/panels), pages (mizu://…), shared design
│                     system and translations (shared/i18n.js)
├── filters/          bundled "Mizu Shield – essential trackers" list
└── icons/            application icon
```

Everything except `mizu-engine`, `mizu-media` (Windows part), `mizu-setup` and `mizu-browser` is platform independent
and unit tested.

## Process and window model

```
mizu.exe (UI thread, single-threaded COM apartment)
 ├── Browser window (Win32, custom frame, Mica)
 │    ├── chrome view   mizu://chrome/   tab strip, toolbar, bookmarks bar        (privileged)
 │    ├── sidebar view  mizu://chrome/?sidebar  vertical tabs, side layout only   (privileged)
 │    ├── tab views     one WebView2 controller per tab (created lazily); two shown in split view (untrusted)
 │    ├── spare view    a pre-created blank view, so a new tab appears instantly
 │    └── flyout popup  mizu://flyout/   menus, panels, prompts (owned popup)     (privileged)
 ├── App window (welcome tour)  same frame, no tabs or toolbar; opens before the browser window on first start
 ├── hidden message window   event queue pump, timers, single-instance IPC
 └── worker threads   filter list downloads & compilation, userscript fetches, media probe

msedgewebview2.exe processes (managed by WebView2): browser, GPU, network, utility, sandboxed renderers per site
```

The browser UI is HTML too, but it runs in dedicated views that only ever load embedded `mizu://` assets. Menus and
suggestion lists live in a separate popup window (the *flyout*), created once per browser window and reused, so menus
open instantly.

### Visual hosting

Views are *composition controllers*. Each view draws into a DirectComposition visual inside our own window
(`mizu-engine::host`); no engine-process window is a child of Mizu's windows. With classic HWND hosting, Windows
coupled our UI thread to the busy engine process, and Mizu froze for seconds with many loading tabs. Because of that,
Mizu forwards input itself:
- mouse, wheel and capture go to the view under the pointer (`host::handle_input`);
- the cursor comes from the view (`CursorChanged`);
- files dragged in go through an OLE drop target (`drop_target.rs`);
- CSS `app-region: drag` in the browser UI answers `WM_NCHITTEST` through `GetNonClientRegionAtPoint`.

Keyboard focus stays with the engine.

With vertical tabs, the same chrome document runs a second time in sidebar mode, to the left of the content. Both
views receive the same coalesced state. The top chrome then hides its tab strip, and the toolbar becomes the title bar,
holding the window buttons. The sidebar width is a setting (160–520 px) that you change by dragging its edge.
The window is shown only when the chrome view reports it is ready (with a 2 s fallback), so it never flashes empty.

## Event flow

1. WebView2 raises a COM event on the UI thread → `mizu-engine` converts it into a typed Rust event and calls the
   `ViewHandler` (`mizu-browser::handler::Dispatcher`).
2. **Decisions** that must be synchronous (navigation guard, request blocking, header injection, document scripts) are
   made against `hot::Hot`, a small state object that is borrowed only briefly.
3. Everything else is pushed onto `queue` and processed from the message loop with exclusive access to `App`.
   Re-entrancy (for example a modal dialog pumping messages while state is borrowed) is impossible by construction:
   modal dialogs run only from `queue::outside`.
4. State changes mark windows dirty; at the end of each pump the browser pushes one coalesced `state` event per window
   to its chrome view.

## UI protocol

`mizu-ui::protocol` defines closed enums for every command (`{"cmd":"new_tab"}`, `{"rid":3,"cmd":"history_query",…}`)
and event (`{"type":"state",…}`, `{"type":"reply","rid":3,…}`). Unknown or malformed messages fail deserialisation and
are dropped. See [SECURITY.md](SECURITY.md) for who may send what.

## Tab model

`mizu-tabs::TabStrip` owns the order and metadata of one window's tabs; the browser mirrors its decisions onto views.

- **Pinned** tabs come first; pinned and normal tabs never interleave.
- **Groups** (`Tab.group`, `TabGroup`) are contiguous runs of tabs with a name, colour and collapsed flag.
- **Spaces** (`Tab.space`, `Space`) split the window into tab sets. Only the active space's tabs are shown; new tabs
  join the active space; activating a tab switches to its space; cycling and Ctrl+1…9 stay within it.
- **Tree tabs** (`Tab.parent`, `tree_collapsed`): a tab opened from a page is inserted after its opener's branch as a
  child. Closing, moving or pinning a tab moves its children up a level.
- **Split view** lives in the window (`BrowserWindow.split`: two tab ids and a ratio). `tab_rect` gives each half its
  rectangle and the divider is dragged in the window procedure.
- Everything above is saved in `WindowSnapshot` (indices for parents and the split) and restored on start.

## Content blocking pipeline

```
filter list texts (cached in profile/filters) ─┐
custom rules ─────────────────────────────────┼─► adblock-rust FilterSet ─► Engine (+ scriptlet/redirect resources)
bundled essential list ───────────────────────┘                (compiled on a worker; swapped in atomically)

NavigationStarting (top level) ─► document check (badware → interstitial)
                                └► document script: site CSS + scriptlets + procedural filters + generic-hiding
                                   reporter + media hub + password capture + userscripts
WebResourceRequested (every request) ─► check(url, page url, type) ─► block | redirect (data:) | strip params | + Sec-GPC

page ─► {mizu_cosmetic: token, classes, ids} ─► hidden_class_id_selectors ─► CSS added to the page
```

- **Site-specific** element hiding and plain `:style()` rules become one stylesheet.
- **Procedural** filters (`:has-text`, `:upward`, `:xpath`, `:matches-*`, actions `:remove`, `:remove-attr`…) are
  passed as JSON to a small in-page engine that re-applies them as the page changes.
- **Generic** rules (`##.ad` on every site) depend on the page's classes and ids, so the page reports new ones in
  batches and Mizu answers with the matching CSS, minus `#@#` exceptions; `$generichide` turns this off.
- The adblock engine is built with `css-validation`, so invalid selectors are dropped at parse time.

## Page scripts

Every top-level navigation gets one document script built by `scripts::document_script`. It is registered before the
navigation; if the document was created before registration finished, the engine runs it once with `ExecuteScript`
so it is never missed. Pages talk back through `chrome.webview.postMessage` with messages that carry a random
per-page token (`mizu_media`, `mizu_pw`, `mizu_cosmetic`, userscript `GM_*`); messages with a wrong token are
ignored.

The page's **context menu** is the engine's, plus entries the browser adds through `ViewHandler::context_items`
(for example "Open video in VLC", handled by `players.rs`).

## Storage layout

```
%LOCALAPPDATA%\Onibi Lab\Mizu\User Data\
├── logs\                       daily logs, 7 kept
├── crashes\                    local crash reports, one text file per crash (never uploaded)
└── Default\
    ├── mizu.db / mizu.db.bak   SQLite (WAL) + last good backup; includes DPAPI-encrypted logins
    ├── settings.toml
    ├── stats.json              lifetime Shield counter
    ├── engine\                 WebView2 user data (cookies, cache, site storage), Default + InPrivate profiles
    ├── filters\                filter lists, resources.json, meta.json
    ├── scripts\                userscripts, @require bundles, GM values
    ├── extensions\             copies of installed extensions
    └── running.lock            crash marker
```

Executables and user data are never mixed. Profiles are separate folders (`--profile Work`).

## Session and crash recovery

The session (all normal windows, tabs, pinned state, groups, spaces, tree, split view, closed-tab stacks, window
placement) is saved to SQLite every few
seconds when it changes and on exit. `running.lock` exists while Mizu runs; if it is present at start, the previous run
crashed and Mizu restores the session or offers a restore button on the new tab page. Restored tabs are *lazy*: their
engine view is created only when the tab is first shown.

## Passwords

1. A small capture script (part of each navigation's document script) notices submitted login forms. It reports the
   origin, username and password over the token-authenticated channel.
2. The browser shows the "Save password?" flyout. Saved secrets are encrypted with DPAPI (`vault.rs`) and stored in
   the `logins` table.
3. The key icon in the omnibox opens the logins flyout. Filling runs a script in the page through `ExecuteScript`.

## Userscript site rules

Each script has user rules (`SiteRules` in `mizu-userscripts`): an "only here" list and a "never here" list.
- When "only" is not empty, the script's own `@match`/`@include` are ignored. The script's `@exclude` and the "never"
  list always apply.
- A rule can be a site name (`youtube.com`, with subdomains), a site with a path (`github.com/user`) or a normal match
  pattern (`https://*.example.org/*`).

## Translations

- `shared/i18n.js` holds the Russian dictionary and a few regular-expression patterns.
- It translates text nodes and the `title`, `placeholder` and `aria-label` attributes, including content added later
  (through a `MutationObserver`).
- The browser sets `data-lang` on `<html>` when it serves a page, so text is translated before the first paint.
- Website data is marked `translate="no"`.

## Updates

```
TIMER_UPDATE (12 s after start, then every 6 h)
  ─► fetch mizu-update.json + .sig from the latest GitHub release
  ─► ed25519 signature check (public key compiled in) ─► is the version newer?
  ─► installed copy? ─► download the installer (URL must be in this repo's releases) ─► SHA-256 check
  ─► "Update to Mizu X? Update now / Later" ─► MizuSetup /S --wait-pid <pid> --relaunch
     (waits for Mizu to exit, replaces the files, starts Mizu again)
```

Only the copy MizuSetup installed (the folder recorded in its uninstall entry) updates itself. A portable copy only announces
new versions. Windows updates the engine independently, and `mizu://about` shows both versions and the update state.

## Installer

`mizu-setup` embeds the release `mizu.exe` at build time (`MIZU_SETUP_PAYLOAD`). Its window is a small frameless
WebView2 page (`ui.html`); if WebView2 cannot start, plain Windows task dialogs are used instead. It:
- installs per user into a folder the user chooses, and creates shortcuts;
- registers Mizu under `HKCU\Software\Clients\StartMenuInternet\Mizu`, with `MizuURL`/`MizuHTML` ProgIDs, so it appears
  in Windows "Default apps";
- writes an uninstall entry and copies itself as `uninstall.exe`. Started under that name it uninstalls, removing only
  the files it installed and the folder only when it is empty.
