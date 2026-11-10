<p align="center">
  <img src="assets/icons/mizu.png" width="96" alt="Mizu logo">
</p>

<h1 align="center">Mizu</h1>
<p align="center"><b>A new way to browse.</b><br>A fast, privacy-focused desktop browser for Windows, written in Rust by <b>Onibi Lab</b>.</p>

<p align="center">
  <img src="docs/screenshots/new-tab.png" width="820" alt="Mizu new tab page">
</p>

> **Status: 0.1.0, early development.** Mizu is usable for everyday browsing, but expect rough edges.
> See [CHANGELOG.md](CHANGELOG.md) for what exists today and [ROADMAP.md](ROADMAP.md) for what comes next.

*水 (mizu) means “water” in Japanese: clear, calm, and quick to take the shape of what you need.*

## Features

**Modern web engine.** Mizu renders pages with Microsoft Edge WebView2, the evergreen Chromium engine that ships with Windows.
Current HTML, CSS, JavaScript, WebAssembly, WebGL/WebGPU, WebRTC and HTTP/3 all work, with hardware video decoding.
Windows keeps the engine updated, including security fixes.

**Mizu Shield: real ad and tracker blocking.**
- Built-in uBlock Origin-compatible filtering on [adblock-rust](https://github.com/brave/adblock-rust). It covers network rules, `$redirect`, `$removeparam`, element hiding and `##+js()` scriptlets.
- Subscribes to uBlock filters, EasyList, EasyPrivacy, Peter Lowe's list and badware lists by default (about 150,000 rules). You can add any list in uBlock or hosts format, and write your own rules.
- Adds the engine's tracking prevention (Basic, Balanced or Strict), Global Privacy Control, and a per-site on/off switch.
- Blocks pages that your filter lists mark as malicious, with an explicit "open anyway".

**Mizu Scripts: a built-in userscript manager.** It is compatible with the Greasemonkey/Tampermonkey script format: `@match`, `@include`, `@exclude`, `@require`, `@run-at`, and the common `GM_*` / `GM.*` API.
Click any `.user.js` link (Greasy Fork, OpenUserJS) to install a script.

**Extensions.** Chromium Manifest V3 extensions, for example uBlock Origin Lite. Install them from a folder, a `.zip` or a `.crx` file.

**Tabs and sessions.**
- Drag to reorder, pin, duplicate, mute, and reopen closed tabs (Ctrl+Shift+T).
- Restores your session automatically and recovers after a crash.
- Tabs restored from a session load only when you open them.
- Sleeping tabs and efficiency mode keep background tabs light.

**Private windows.** They use a separate in-memory profile. Nothing is written to history, and all data is discarded when the last private window closes.

**History, bookmarks and downloads.**
- Searchable history, with removal of single entries or a whole time period.
- Bookmark folders, a bookmarks bar, drag-and-drop ordering.
- A download manager with speed, time left, pause/resume, retry, and "show in folder".

**Settings.** 10 categories: General, Appearance, Search, Privacy & Shield, Security, Downloads, Tabs, Performance, Media and Advanced. That includes clearing data by category and time range, site permissions, and custom search engines with `@keyword` shortcuts.

**Design.**
- Light, dark and system themes, and five palettes: Mizu, Washi, Matcha, Sumi and Sakura.
- Any accent colour, compact/comfortable/spacious density, a soft or crisp style.
- Mica translucency on Windows 11, and motion that respects "reduce motion".

**Media diagnostics.** `mizu://media` shows which codecs the engine can play, which decoders the system has, and whether hardware acceleration is available.

**No telemetry.** Mizu sends nothing to Onibi Lab. Your data stays in your profile folder.

<p align="center">
  <img src="docs/screenshots/settings.png" width="780" alt="Settings">
</p>

## System requirements

- Windows 10 (version 1809 or later) or Windows 11, x64
- [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/). It is preinstalled on Windows 11 and on most Windows 10 machines. Mizu tells you if it is missing.

Linux and macOS are planned (see the roadmap).

## Building

You need Rust 1.85 or newer with the MSVC toolchain (`x86_64-pc-windows-msvc`) and the Visual Studio Build Tools (C++ workload and Windows SDK).

```bash
git clone https://github.com/senzukisu/mizu.git
```

```bash
cd mizu
```

```bash
cargo build --release
```

The browser is at `target/release/mizu.exe`. Run the tests with:

```bash
cargo test --workspace
```

Development details are in [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

### Command line

| Argument | Effect |
|---|---|
| `mizu <url>…` | Open the URLs (in the running instance if Mizu is already open) |
| `--private` | Open a private window |
| `--profile <name>` | Use a separate profile |
| `MIZU_USER_DATA=<dir>` | Store all profiles in `<dir>` (portable or test setups) |
| `MIZU_LOG=debug` | Verbose logging to `…\User Data\logs` |

## Documentation

- [Architecture](docs/ARCHITECTURE.md): crates, processes, data flow, and why WebView2
- [Security](docs/SECURITY.md): trust model, sandboxing, internal pages, IPC
- [Codecs](docs/CODECS.md): what plays, hardware decoding, licensing
- [Development](docs/DEVELOPMENT.md) and [Testing](docs/TESTING.md)
- [Roadmap](ROADMAP.md) and [Changelog](CHANGELOG.md)

## License

Mizu is licensed under the [Mozilla Public License 2.0](LICENSE).
Content blocking uses adblock-rust by Brave Software (MPL-2.0), and scriptlet resources come from brave/adblock-resources (MPL-2.0).
Filter lists are downloaded at runtime from their maintainers and keep their own licenses.
Mizu is not affiliated with uBlock Origin, Tampermonkey, Microsoft or Brave.

© Onibi Lab
