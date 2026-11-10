# Mizu architecture

## Engine choice

| Option | Compatibility | Codecs | Updates & security | Footprint | Verdict for 0.1 |
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
├── mizu-storage      SQLite: history, bookmarks, downloads, permissions, session; migrations, backup, recovery
├── mizu-tabs         tab strip model (order, pinning, closed-tab stack, snapshots) – pure logic
├── mizu-privacy      Mizu Shield: adblock-rust engine, filter list store/updater, permissions catalogue, stats
├── mizu-userscripts  Mizu Scripts: metadata parser, match patterns, script store, GM_* bootstrap
├── mizu-media        media diagnostics (Media Foundation decoders, DXGI GPUs)
├── mizu-ui           UI protocol (typed JSON commands/events) + embedded HTML/CSS/JS assets
├── mizu-engine       the only crate that touches WebView2/COM: Engine, View, Download, events
└── mizu-browser      the application: windows, tabs, flyouts, wiring, internal page data
assets/
├── ui/               chrome (tab strip/toolbar), flyout (menus/panels), pages (mizu://…), shared design system
├── filters/          bundled "Mizu Shield – essential trackers" list
└── icons/            application icon
```

Everything except `mizu-engine`, `mizu-media` (Windows part) and `mizu-browser` is platform independent and unit tested.

## Process and window model

```
mizu.exe (UI thread, single-threaded COM apartment)
 ├── Browser window (Win32, custom frame, Mica)
 │    ├── chrome view   mizu://chrome/   tab strip, toolbar, bookmarks bar        (privileged)
 │    ├── tab views     one WebView2 controller per tab (created lazily)          (untrusted)
 │    └── flyout popup  mizu://flyout/   menus, panels, prompts (owned popup)     (privileged)
 ├── hidden message window   event queue pump, timers, single-instance IPC
 └── worker threads   filter list downloads & compilation, userscript fetches, media probe

msedgewebview2.exe processes (managed by WebView2): browser, GPU, network, utility, sandboxed renderers per site
```

The browser UI is HTML too, but it runs in dedicated views that only ever load embedded `mizu://` assets. Web content
is a native child window that HTML cannot overlap, so menus and suggestion lists live in a separate popup window (the
*flyout*). That window is created once per browser window and reused, so menus open instantly.

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

## Content blocking pipeline

```
filter list texts (cached in profile/filters) ─┐
custom rules ─────────────────────────────────┼─► adblock-rust FilterSet ─► Engine (+ scriptlet/redirect resources)
bundled essential list ───────────────────────┘                (compiled on a worker; swapped in atomically)

NavigationStarting (top level) ─► document check (badware → interstitial)
                                └► cosmetic CSS + scriptlets + userscripts ─► AddScriptToExecuteOnDocumentCreated
WebResourceRequested (every request) ─► check(url, page url, type) ─► block | redirect (data:) | strip params | + Sec-GPC
```

## Storage layout

```
%LOCALAPPDATA%\Onibi Lab\Mizu\User Data\
├── logs\                       daily logs, 7 kept
└── Default\
    ├── mizu.db / mizu.db.bak   SQLite (WAL) + last good backup
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

The session (all normal windows, tabs, pinned state, closed-tab stacks, window placement) is saved to SQLite every few
seconds when it changes and on exit. `running.lock` exists while Mizu runs; if it is present at start, the previous run
crashed and Mizu restores the session or offers a restore button on the new tab page. Restored tabs are *lazy*: their
engine view is created only when the tab is first shown.

## Updates (planned for 0.2)

0.1.0 contains no updater. The design for 0.2 is:
- a release manifest (`version`, `url`, `sha256`, `min_engine`) signed with an Onibi Lab ed25519 key, whose public key is
  compiled into the binary;
- HTTPS download, hash plus signature verification before anything is executed, an atomic swap on restart, and
  automatic rollback if the new version fails to reach the message loop.

The engine itself is updated by Windows independently of Mizu. `mizu://about` shows the running engine version.
