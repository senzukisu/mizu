# Testing Mizu

## Automated tests

```bash
cargo test --workspace
```

| Crate | What is covered |
|---|---|
| mizu-core | URL vs search classification (IDN, ports, IPs, `javascript:` refusal), search URLs, origins, profile name sanitising, internal page registry |
| mizu-settings | round trip, defaults for partial files, corrupt file recovery, sanitising invalid values |
| mizu-storage | migrations, history (record, search, suggestions, delete, clear by period), bookmarks (folders, move, cycles, protection of roots), downloads lifecycle, permissions, session, **corruption recovery from backup** |
| mizu-tabs | activation after close, reopen position, pinned ordering rules, drag constraints, duplicate, cycling and shortcuts, snapshot round trip |
| mizu-privacy | third-party blocking, exceptions, `$redirect`, `$removeparam`, allowlist, cosmetic filters and scriptlets, hosts files, bundled list, HTTPS-only downloads, list validation, path safety |
| mizu-userscripts | metadata parsing, `@match`/`@include` (globs, regex, `.tld`), install/update/enable/remove, value storage limit, bootstrap generation and escaping |
| mizu-media | Media Foundation probe runs and finds an H.264 decoder |
| mizu-ui | protocol parsing and rejection of unknown commands, asset resolution and path traversal |
| mizu-browser | shortcuts, launch args, omnibox suggestions and keyword engines, data URL decoding, allowlist matching, download file naming, CRX parsing, token generation |

Syntax check of the UI scripts (requires Node.js):

```bash
for f in assets/ui/chrome/*.js assets/ui/flyout/*.js assets/ui/pages/*.js assets/ui/shared/*.js; do node --check "$f" || exit 1; done
```

## Manual checklist (run before every release)

Use a fresh profile: `MIZU_USER_DATA=%TEMP%\mizu-test`.

1. **Start and close.** Mizu starts in under 2 s, closing asks for confirmation with several tabs open, and the next
   start restores the tabs.
2. **Modern sites.** YouTube (video plays, 1080p/4K, fullscreen, no pre-roll ads), GitHub (sign in), Discord web,
   Reddit, Google, Wikipedia, Twitch (live stream), an online shop checkout, a heavy SPA (Figma or Notion).
3. **JS, CSS and WebAssembly.** html5test.com, a WebAssembly demo, WebGL and WebGPU samples.
4. **Tabs.** Open 30 tabs, drag to reorder, pin, duplicate, mute, close with the middle button, reopen with Ctrl+Shift+T,
   Ctrl+1…9 and Ctrl+Tab.
5. **Session and crash.** Kill the process (`taskkill /IM mizu.exe /F`), start again, and check the tabs come back
   (or the restore button appears).
6. **History.** Search, delete an entry, clear the last hour.
7. **Bookmarks.** Ctrl+D, rename, move to a folder, drag on the manager page, the bookmarks bar.
8. **Downloads.** A large file: pause, resume, cancel, retry, open, show in folder, change the folder, "ask where to save".
9. **Themes.** Light, dark and system, every palette, an accent colour, density and style; restart and check they persist.
10. **Settings persist** after a restart.
11. **Private window.** No history entries, cookies are not shared with normal windows, and closing the last private
    window signs you out of sites logged in there.
12. **Clearing data.** Clear cookies for "last hour" and check you are signed out.
13. **Shield.** `https://d3ward.github.io/toolz/adblock` or a similar test page, the per-site switch, a custom rule,
    adding a list.
14. **Media.** `mizu://media`: H.264/VP9/AV1 show Hardware on capable GPUs, and audio plays.
15. **Sandbox.** Process Explorer shows renderer processes running with an untrusted/low integrity level.
16. **Internal page guard.** In the DevTools console of a website, `location = 'mizu://settings/'` must do nothing.
17. **Memory.** With 20 tabs open, after the sleep delay background tabs are suspended (Task manager, Shift+Esc).
18. **Userscripts.** Install a script from greasyfork.org, check it runs, that `GM_setValue` persists across reloads, then
    disable and remove it.
19. **Extensions.** Install uBlock Origin Lite from its `.zip`, open its options, disable and remove it.
