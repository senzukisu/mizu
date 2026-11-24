# Testing Mizu

Two layers: automated tests that run on every push, and a manual checklist before every release.

**Contents:** [Automated tests](#automated-tests) · [Looking at the interface without clicking](#looking-at-the-interface-without-clicking) ·
[Release checklist](#release-checklist)

---

## Automated tests

```bash
cargo test --workspace
```

```bash
node scripts/check-ui.mjs
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

CI (`.github/workflows/ci.yml`) runs the interface check on Linux and clippy plus the tests on Windows.

| Crate | What is covered |
|---|---|
| mizu-core | address vs search (IDN, ports, IPs, refusing `javascript:`), search URLs, origins, profile names, internal pages, calculator and unit conversion |
| mizu-settings | round trip, defaults for partial files, recovery from a corrupt file, sanitising bad values |
| mizu-storage | migrations, history, bookmarks (folders, moves, cycles, protected roots), new tab tiles and folders, downloads, permissions, session, logins, import from other browsers, recovery from backup |
| mizu-tabs | activation after close, reopen position, pinned ordering, drag rules, duplicate, cycling, groups, spaces, tree tabs, snapshots |
| mizu-privacy | third-party blocking, exceptions, `$redirect`, `$removeparam`, allowlist, site, generic and procedural cosmetic filters, scriptlets, hosts files, list validation |
| mizu-userscripts | metadata, `@match`/`@include`, install/update/remove, storage limit, bootstrap escaping, link rewriting, several scripts per file, update schedule, site rules |
| mizu-update | signature check, tampered manifests, wrong key, installer hash, version comparison |
| mizu-ui | protocol parsing, unknown commands refused, asset lookup, path traversal |
| mizu-browser | shortcuts and key parsing, launch arguments, address bar suggestions, cookie cleanup, media links, data URLs, CRX parsing, DPAPI, password CSV |
| mizu-media | the Media Foundation probe finds an H.264 decoder |

---

## Looking at the interface without clicking

To check a change visually, start Mizu with a **test profile** and put it in the state you need through files and URL
hooks instead of input:

1. `MIZU_USER_DATA=%TEMP%\mizu-test` — a fresh, disposable profile.
2. Write `Default\settings.toml` before the start (e.g. `[appearance] tab_layout = "side"`,
   `[general] welcome_version = 3` to skip the tour).
3. For tabs, start once, close, then write a session into `Default\mizu.db` (`session` table, JSON with `tabs`,
   `groups`, `spaces`, `split`, `parent`).
4. Open a state directly: `mizu://settings/#q=…`, `mizu://newtab/#customize`, `mizu://newtab/#folder=<id>`,
   `mizu://welcome/#2`.
5. Capture the window (e.g. with `PrintWindow`) and stop only the process you started.

---

## Release checklist

Use a fresh profile. Tick each line.

### Start and tabs

- [ ] Starts in under 2 s; the next start restores the tabs; after `taskkill /IM mizu.exe /F` it offers to restore.
- [ ] First start of a new profile shows the welcome tour once, in its own window; the browser opens after it.
- [ ] Tabs: drag, pin, duplicate, mute, middle-click close, Ctrl+Shift+T, Ctrl+1…9, Ctrl+Tab, **Unload tab**.
- [ ] 30+ heavy sites opened quickly: the window stays responsive; the strip scrolls without shaking.
- [ ] Side tabs: one top row with window buttons; dragging the empty toolbar moves the window; resize and collapse;
  pinned grid drag in/out.
- [ ] Tree tabs: a link opened in a new tab sits under its page; fold/unfold; close a branch.
- [ ] Groups: create, rename, colour, collapse by click, menu by right-click.
- [ ] Spaces: create, switch (top and side), move a tab, delete a space; restored after restart.
- [ ] Split view: open, drag the divider, swap, close; restored after restart.
- [ ] Private window: no history, separate cookies, signed out after closing.

### Address bar and pages

- [ ] Suggestions, `@g` keyword, calculator, `10 km in mi`, `>` quick actions, switch to tab.
- [ ] Find on page: next/previous, match case, Esc.
- [ ] Bookmarks: Ctrl+D, folders, the bar appears with its first bookmark, right-click menu.
- [ ] New tab: add, edit, reorder tiles; make a folder by dropping a tile on another; customize panel.
- [ ] History search and clearing; downloads pause/resume/retry, **Open with…**.
- [ ] Import from an installed browser: Mizu's bookmarks bar is untouched unless the box is ticked.

### Privacy and security

- [ ] Shield: an ad test page; per-site switch; a custom rule; a generic `##.class` rule; a `:has-text()` rule.
- [ ] Site settings: cookies, Keep me signed in, Forget this site, permissions, zoom reset.
- [ ] Per-site zoom is remembered and applied when coming back.
- [ ] Passwords: save prompt after sign-in, fill from the key, CSV import/export.
- [ ] Search suggestions never appear in a private window.
- [ ] In a site's DevTools console, `location = 'mizu://settings/'` does nothing.
- [ ] A link to an app (e.g. `mailto:`) asks first.

### Media

- [ ] YouTube 1080p/4K, fullscreen, no pre-roll ads; Twitch live.
- [ ] `mizu://media`: H.264/VP9/AV1 "Hardware" on capable GPUs.
- [ ] Media panel: play/pause/next, volume per site, media keys, one sound at a time.
- [ ] Picture in picture: button, Alt+Shift+P, next/previous and skip buttons.
- [ ] With VLC or mpv installed: right-click a video → **Open video in …**.

### Scripts, extensions, shortcuts

- [ ] Install a userscript from Greasy Fork; `GM_setValue` persists; **Where it runs** limits it.
- [ ] Install an extension with **Add to Mizu**; options survive a restart; disable and remove.
- [ ] Settings → Shortcuts: change "New tab"; new keys work, old ones don't; reset.

### Install and update

- [ ] MizuSetup into a custom folder: Start menu entry, listed in Default apps, `uninstall.exe` removes it.
- [ ] An installed copy sees a higher version: "Update now / Later"; "Update now" restarts into it with the tabs.
- [ ] Rollback: run `MizuSetup-X.Y.Z.exe /S --relaunch` with `MIZU_TEST_FAIL_START=1` set — the installer restores
  `mizu.prev` and leaves `update-failed`; the next normal start says the update was rolled back.
