# Mizu security model

## Reporting vulnerabilities

Please report security issues privately via GitHub Security Advisories on the repository, not in public issues.

## Principles

- Use the engine's sandbox and site isolation. Never weaken them.
- Use no custom cryptography: TLS is done by the engine and by Windows (SChannel), and future signature or encryption
  work will use audited crates.
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

Pop-ups without a user gesture are blocked. Links to external applications go through the engine's confirmation
dialog, and schemes that have been used for attacks (`ms-msdt:`, `search-ms:`, `ms-officecmd:`, `shell:` …) are refused.

## Userscripts and extensions

- Userscripts run in the page's context, which is how Greasemonkey-style scripts work. Their `GM_*` functions live in a
  closure that page scripts cannot reach.
- Writes to a script's stored values go through `chrome.webview.postMessage`. Each message carries a random token that
  exists only inside the closure; the browser accepts writes only for (token, script) pairs it issued to that tab.
  Storage is capped at 4 MB per script.
- The install page shows the source, the sites the script runs on, unsupported APIs and a warning before installing.
- Extensions are installed only through a file dialog, copied into the profile, and unpacked with zip-slip protection
  and a size limit. Private windows run without extensions.

## Downloads

Files are saved through the engine, so Windows Mark-of-the-Web and SmartScreen checks (when enabled) apply.
Mizu never opens a downloaded file automatically.

## Local data

The profile lives under `%LOCALAPPDATA%`, which is protected by Windows user ACLs. The engine stores cookies
(encrypted with DPAPI by Chromium) in `engine\`. Mizu 0.1 has no password manager; engine password saving is off by
default.

## Telemetry

None. Crash dumps produced by the engine stay on the machine (custom crash reporting is enabled, so nothing is
uploaded to Microsoft by Mizu's choice). Note that SmartScreen, when enabled, checks addresses with Microsoft's service;
the setting explains this.
