# Testing Mizu

## Automated tests

```bash
cargo test --workspace
```

| Crate | What is covered |
|---|---|
| mizu-core | URL vs search classification (IDN, ports, IPs, `javascript:` refusal), search URLs, origins, profile name sanitising, internal page registry |
| mizu-settings | round trip, defaults for partial files, corrupt file recovery, sanitising invalid values |
| mizu-storage | migrations, history (record, search, suggestions, delete, clear by period), bookmarks (folders, move, cycles, protection of roots), downloads lifecycle, permissions, session, saved logins, **corruption recovery from backup** |
| mizu-tabs | activation after close, reopen position, pinned ordering rules, drag constraints, duplicate, cycling and shortcuts, snapshot round trip |
| mizu-privacy | third-party blocking, exceptions, `$redirect`, `$removeparam`, allowlist, cosmetic filters and scriptlets, hosts files, bundled list, HTTPS-only downloads, list validation, path safety |
| mizu-userscripts | metadata parsing, `@match`/`@include` (globs, regex, `.tld`), install/update/enable/remove, value storage limit, bootstrap generation and escaping, GitHub/Gist/GitLab/Greasy Fork link rewriting, several scripts in one file, update scheduling, per-script site rules |
| mizu-media | Media Foundation probe runs and finds an H.264 decoder |
| mizu-update | signature verification, tampered manifests, wrong key, installer hash, version comparison |
| mizu-ui | protocol parsing and rejection of unknown commands, asset resolution and path traversal |
| mizu-browser | shortcuts, launch args, omnibox suggestions and keyword engines, data URL decoding, allowlist matching, download file naming, CRX parsing, token generation, DPAPI round trip, password CSV import/export (Chrome, Firefox, Bitwarden) |

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
19. **Extensions.** Install an extension with "Add to Mizu" on the Chrome Web Store, open its options, disable and remove it.
20. **Passwords.**
    1. Sign in to a site and check the save prompt appears.
    2. Sign out, then fill the login from the key icon.
    3. Export to CSV and import it again.
21. **Script sites.** Open "Where it runs" for a script, limit it to one site, check it runs only there, then add a
    site to "Never run on".
22. **Vertical tabs.** Switch the layout in Settings → Appearance, then:
    - resize the sidebar and collapse it;
    - check that "New tab" stays under the last tab.
23. **Picture in picture.** Hover a video on YouTube and press PiP, then try Alt+Shift+P.
    - In the floating window, check the previous/next video buttons and the 10-second skips.
    - Check that the button position and compact button settings work.
24. **Many tabs.** Open 30+ heavy tabs quickly and check that:
    - the window stays responsive and Explorer is unaffected;
    - the strip scrolls sideways without shaking;
    - closing tabs is smooth.
25. **Shortcuts.** Change "New tab" in Settings → Shortcuts. Check that the new keys work, the old ones don't, then reset.
26. **Search suggestions.** Type a query and check the engine's suggestions appear; check that none appear in a private window.
27. **Language.** Switch to Russian in Settings → General and check every page and menu is translated.
28. **Installer and update.**
    1. Install with MizuSetup into a custom folder and start Mizu from the Start menu.
    2. Check that Default apps lists Mizu, then uninstall.
    3. Publish a test release with a higher version and check that "Restart to update" installs it.
