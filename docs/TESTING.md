# Testing Mizu

## Automated tests

```bash
cargo test --workspace
```

| Crate | What is covered |
|---|---|
| mizu-core | URL vs search classification (IDN, ports, IPs, `javascript:` refusal), search URLs, origins, profile names, internal pages |
| mizu-settings | round trip, defaults for partial files, corrupt file recovery, sanitising invalid values |
| mizu-storage | migrations, history, bookmarks (folders, moves, cycles, protected roots), downloads, permissions, session, saved logins, **recovery from backup** |
| mizu-tabs | activation after close, reopen position, pinned ordering, drag constraints, duplicate, cycling, snapshots |
| mizu-privacy | third-party blocking, exceptions, `$redirect`, `$removeparam`, allowlist, cosmetic filters, scriptlets, hosts files, HTTPS-only downloads, list validation |
| mizu-userscripts | metadata, `@match`/`@include`, install/update/remove, storage limit, bootstrap escaping, link rewriting (GitHub, Gist, GitLab, Greasy Fork), several scripts per file, update scheduling, site rules |
| mizu-update | signature verification, tampered manifests, wrong key, installer hash, version comparison |
| mizu-ui | protocol parsing, rejection of unknown commands, asset resolution, path traversal |
| mizu-browser | shortcut bindings and key parsing, launch args, omnibox suggestions and search-engine suggestions, data URLs, CRX parsing and store keys, DPAPI, password CSV import/export |
| mizu-media | the Media Foundation probe finds an H.264 decoder |

Check the UI scripts' syntax (requires Node.js):

```bash
for f in assets/ui/*/*.js; do node --check "$f" || exit 1; done
```

Lint:

```bash
cargo clippy --workspace
```

## Manual checklist

Run it before every release with a fresh profile (`MIZU_USER_DATA=%TEMP%\mizu-test`).

### Start, tabs and windows
- [ ] Mizu starts in under 2 s, maximised; the next start restores the tabs; after `taskkill /IM mizu.exe /F` it offers to restore them.
- [ ] Tabs:
  - drag, pin, duplicate, mute, middle-click close;
  - Ctrl+Shift+T, Ctrl+1…9, Ctrl+Tab;
  - **Unload tab** and **Unload other tabs** (the tab reloads when opened).
- [ ] Open 30+ heavy sites quickly:
  - the window stays responsive and Explorer is unaffected;
  - the strip scrolls sideways without shaking;
  - closing tabs is smooth.
- [ ] Vertical tabs (Settings → Appearance):
  - one top row with the window buttons;
  - dragging the empty toolbar moves the window;
  - the sidebar resizes and collapses;
  - "New tab" stays under the last tab.
- [ ] Menus and panels: a second click on their button closes them.
- [ ] Private window: no history; cookies are separate; closing the last one signs you out of sites used there.

### Sites and media
- [ ] YouTube (1080p/4K, fullscreen, no pre-roll ads), GitHub sign-in, Discord, Reddit, Twitch live, a shop checkout, Figma or Notion.
- [ ] html5test.com, a WebAssembly demo, WebGL and WebGPU samples.
- [ ] `mizu://media`: H.264/VP9/AV1 show "Hardware" on capable GPUs.
- [ ] Picture in picture on YouTube:
  - the hover button and Alt+Shift+P work;
  - previous/next video and the 10-second skips work in the floating window;
  - the button position and compact button settings take effect.
- [ ] Drag a file from Explorer onto an upload field on a page.

### Data
- [ ] History: search, delete an entry, clear the last hour.
- [ ] Bookmarks:
  - Ctrl+D, rename, move to a folder;
  - the bar appears with its first bookmark;
  - right-click → Open in new tab / Delete.
- [ ] Downloads: pause, resume, cancel, retry, open, show in folder, change the folder, "ask where to save".
- [ ] Settings, themes, palettes and the language survive a restart.
- [ ] Clear cookies for "last hour" and check you are signed out.

### Privacy and security
- [ ] Shield: an ad-block test page, the per-site switch, a custom rule, adding a list.
- [ ] Passwords:
  - the save prompt appears after sign-in, and filling from the key icon works;
  - CSV export and import;
  - Settings → Security → Open password manager.
- [ ] Search suggestions appear while typing, and never in a private window.
- [ ] In a website's DevTools console, `location = 'mizu://settings/'` does nothing.
- [ ] Process Explorer shows renderers running at untrusted/low integrity.

### Scripts, extensions, shortcuts
- [ ] Install a userscript from greasyfork.org:
  - `GM_setValue` persists across reloads;
  - limit it to one site with **Where it runs**, then add a site to **Never run on**.
- [ ] Install an extension with "Add to Mizu":
  - change an option, restart, and the option is kept;
  - disable it, then remove it.
- [ ] Settings → Shortcuts: change "New tab"; the new keys work and the old ones don't; reset.

### Install and update
- [ ] MizuSetup into a custom folder:
  - it starts from the Start menu;
  - Default apps lists Mizu;
  - `uninstall.exe` removes it.
- [ ] Publish a test release with a higher version. An installed copy asks "Update now / Later", and "Update now" restarts into the new version.
