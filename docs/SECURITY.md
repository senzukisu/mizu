# Mizu security model

What Mizu protects, how, and what it sends over the network.

**In short**

- Pages run in Chromium's sandbox, isolated per site; Mizu never weakens it.
- Web pages cannot reach Mizu's own pages or commands; every message from a page needs a per-page secret token.
- Passwords and kept session cookies are encrypted with your Windows account (DPAPI).
- Updates are signed (ed25519) and checked twice before anything runs, and only after you agree.
- No telemetry. Crash reports stay on your PC.

**Contents:** [Reporting a vulnerability](#reporting-a-vulnerability) · [Principles](#principles) ·
[Processes and sandboxing](#processes-and-sandboxing) · [Trust levels](#trust-levels) · [Internal pages](#internal-pages) ·
[Messages from pages](#messages-from-pages) · [Network](#network) · [Permissions](#permissions) ·
[Pop-ups and external applications](#pop-ups-and-external-applications) · [Userscripts and extensions](#userscripts-and-extensions) ·
[Downloads](#downloads) · [Media players](#media-players) · [Local data](#local-data) · [Passwords](#passwords) ·
[Updates](#updates) · [Installer](#installer) · [Search suggestions](#search-suggestions) · [Telemetry](#telemetry)

## Reporting a vulnerability

Please report security issues **privately** through GitHub Security Advisories on the repository
(Security → Report a vulnerability), not in public issues. Include the Mizu version (`mizu://about`) and steps to
reproduce.

## Principles

- Use the engine's sandbox and site isolation. Never weaken them.
- Use no custom cryptography. TLS is done by the engine and by Windows (SChannel), passwords are encrypted by Windows
  DPAPI, and update signatures use `ed25519-dalek`.
- Web content is untrusted. Mizu's own UI is the only privileged web content, and it never loads anything from the network.
- Fail closed: unknown messages are dropped, unknown permissions are denied, invalid certificates are rejected.

## Processes and sandboxing

Pages run in Chromium renderer processes that are sandboxed and isolated per site. They are managed by the WebView2
runtime; Mizu does not change sandbox flags. The browser process (`mizu.exe`) runs as a normal user (`asInvoker`) and
executes no web content itself.

## Trust levels

| Surface | Content | May send |
|---|---|---|
| Chrome view (`mizu://chrome/`) | Embedded assets only | All UI commands |
| Flyout view (`mizu://flyout/`) | Embedded assets only | All UI commands |
| Internal pages (`mizu://settings/` …) | Embedded assets only | Data commands (not window/flyout/prompt commands) |
| Web pages | Anything | Only the userscript channel, authenticated by a per-page token |

The trust level is decided from the **view's role** (known to the browser) **and** the message's source URL reported
by the engine. A web page cannot claim to be the chrome.

## Internal pages

- They are served from assets compiled into the executable, so editing files on disk cannot change them.
- A strict CSP is applied: `script-src 'self'`, `connect-src 'none'`, `frame-ancestors 'none'`. Data from websites
  (titles, URLs) is inserted with `textContent` only, never parsed as HTML.
- **Navigation guard:** a tab may enter `mizu://` only if the browser itself started that navigation (address bar, menu),
  if it comes from another internal page, or if it is a back/forward or reload history navigation. Web pages cannot
  navigate to or frame internal pages. `mizu://chrome` and `mizu://flyout` can never be loaded in a tab.
- `mizu://` resources requested by web pages get a 403 response.
- Engine internals (`edge://`, `chrome://`) and unknown schemes are refused.

## Messages from pages

Web pages may talk to Mizu only through `chrome.webview.postMessage`, and only for a few features: media controls,
the password-save prompt, generic element hiding and userscript storage. Each feature gets a random token per page,
known only to the script Mizu injected; a message with a missing or wrong token is ignored. Even with a valid token a
page can only *report* (a playing track, a submitted login, the classes it uses) — it cannot run commands.

## Network

- TLS 1.2/1.3 and certificate verification are handled by the engine. Certificate errors show the engine's error page
  with no "proceed anyway".
- Mizu's own downloads (filter lists, userscripts) use HTTPS only, through the Windows certificate store, with size
  limits. Downloaded filter lists are validated before replacing the previous copy.
- Userscripts requested over `http://` are upgraded to HTTPS.

## Permissions

Camera, microphone, location, notifications, clipboard reading and other powerful features are **asked** by default.
Decisions are stored only when the user ticks "Remember". Private windows keep decisions in memory only.
The engine is told not to persist permission decisions itself, so Mizu's list is the single source of truth.
Requests whose kind cannot be described accurately are denied.

## Pop-ups and external applications

Pop-ups without a user gesture are blocked. Links to external applications go through Mizu's own prompt, and schemes that have been used for attacks (`ms-msdt:`, `search-ms:`, `ms-officecmd:`, `shell:` …) are refused.

## Userscripts and extensions

- Userscripts run in the page's context, which is how Greasemonkey-style scripts work. Their `GM_*` functions live in a
  closure that page scripts cannot reach.
- Writes to a script's stored values go through `chrome.webview.postMessage`. Each message carries a random token that
  exists only inside the closure; the browser accepts writes only for (token, script) pairs it issued to that tab.
  Storage is capped at 4 MB per script.
- The install page shows the source, the sites the script runs on, unsupported APIs and a warning before installing.
- Userscript sources are fetched over HTTPS only. Links to GitHub, GitLab, Gist and similar hosts are rewritten to
  their raw-file addresses. Automatic updates use `@updateURL`/`@downloadURL`.
- Extensions come from a file the user picks, or from the Chrome Web Store when the user presses "Add to Mizu" on a
  store page. CRX files are parsed and unpacked with zip-slip protection and a size limit, then copied into the
  profile. Private windows run without extensions.

## Downloads

Files are saved through the engine, so Windows Mark-of-the-Web and SmartScreen checks (when enabled) apply.
Mizu never opens a downloaded file automatically.

## Media players

"Open video in …" starts an installed player (VLC, mpv, PotPlayer, MPC) found in the usual install locations. Mizu
passes exactly one argument: an `http(s)` address without spaces or quotes. Nothing else from the page reaches the
command line, and it only happens when you pick the menu item.

## Local data

The profile lives under `%LOCALAPPDATA%`, which is protected by Windows user ACLs. The engine stores cookies
(encrypted with DPAPI by Chromium) in `engine\`.

- **Session cookies.** Chromium drops cookies without an expiry date on exit. When "Continue where I left off" is on,
  Mizu saves them to `session-cookies.bin`, encrypted with DPAPI, and adds them back before the first page loads. They
  are never saved from private windows.
- **Third-party cookies** are blocked by default with Chromium's own third-party cookie mode, so embedded frames get
  no cookies.
- **Links to other apps.** External schemes always go through Mizu's prompt. Schemes known to be abused
  (`ms-msdt:`, `search-ms:`, `ms-officecmd:`, `ms-cxh:`, `shell:`, `file:`, `javascript:`, `vbscript:`,
  `ms-appinstaller:`) are never opened. "Always allow" is stored per site and scheme, and never in private windows.

## Passwords

- Secrets are encrypted with `CryptProtectData` (DPAPI, bound to the Windows user) with additional entropy, and stored
  as blobs in `mizu.db`. Usernames and origins are not encrypted, so the list can be searched.
- The capture script runs in pages, but it can only *suggest* a login. The message must carry the per-page token, and
  nothing is stored until the user presses Save in Mizu's own prompt.
- Logins are offered only for the exact origin they were saved for, and are filled only when the user picks one.
- CSV export writes passwords in plain text, as every browser does. Mizu warns about this before exporting.
- The engine's own password saving is off, so there is only one password store.

## Updates

- The release manifest is signed with an Onibi Lab ed25519 key kept offline. Only the public key is in the binary.
- The installer URL must start with `https://github.com/senzukisu/mizu/releases/download/`, and its SHA-256 must match the
  signed manifest. Nothing runs before both checks pass.
- The update runs only after the user agrees ("Update now"), as the same user and without elevation.

## Installer

- Setup never asks for elevation; a folder the user cannot write to is refused with an explanation.
- Uninstall deletes only `mizu.exe`, `uninstall.exe` and their temporary copies, then the folder if it is empty, so a
  folder chosen by mistake never loses other files. Browsing data is deleted only when the user ticks the option.

## Search suggestions

- With **Search suggestions** on, what you type in the address bar is sent to the selected search engine's
  suggestion service (and nowhere else), after a 120 ms pause in typing.
- Text that looks like an address is never sent, and private windows never send anything.
- Engines without a public suggestion service get no remote suggestions. Mizu never asks a different provider.
- History and bookmark suggestions are computed locally.

## Telemetry

None. Mizu's own crash reports (version and where it failed) are written to the `crashes` folder in Mizu's user data and never
sent. Crash dumps produced by the engine stay on the machine too (custom crash reporting is enabled, so nothing is
uploaded to Microsoft by Mizu's choice). Note that SmartScreen, when enabled, checks addresses with Microsoft's service;
the setting explains this.
