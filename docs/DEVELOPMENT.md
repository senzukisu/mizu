# Developing Mizu

How to build Mizu, change it safely and ship a release. For how the parts fit together see
[ARCHITECTURE](ARCHITECTURE.md); for tests see [TESTING](TESTING.md).

**Contents:** [Cheat sheet](#cheat-sheet) · [Set up](#set-up) · [Run](#run) · [Where things live](#where-things-live) ·
[Recipes](#recipes) · [Code standards](#code-standards) · [Before a commit](#before-a-commit) ·
[Releasing](#releasing) · [The signing key](#the-signing-key)

---

## Cheat sheet

| Task | Command |
|---|---|
| Build / run (debug) | `cargo build` / `cargo run` |
| Run with a throwaway profile | `MIZU_USER_DATA="$TEMP/mizu-dev" MIZU_LOG=mizu=debug cargo run` |
| Open a URL in the running copy | `cargo run -- https://example.com` |
| Build next to a running copy | `CARGO_TARGET_DIR=target/alt cargo run` |
| Check the interface scripts | `node scripts/check-ui.mjs` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Test | `cargo test --workspace` |
| Release build | `powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -Notes "…"` |
| Free disk space | `cargo clean` (a debug build is ~6 GB, release ~2 GB) |

---

## Set up

- Windows 10/11 x64.
- Rust stable ≥ 1.85, toolchain `x86_64-pc-windows-msvc`.
- Visual Studio Build Tools → "Desktop development with C++" (MSVC and the Windows SDK; `rc.exe` builds the icon and
  manifest).
- Microsoft Edge WebView2 Runtime (already on Windows 11).
- Node.js — for `scripts/check-ui.mjs`.

---

## Run

- **Never use your real profile while developing.** `MIZU_USER_DATA` points Mizu at another folder; delete it to start
  fresh.
- **Logs:** `<user data>/logs/mizu.YYYY-MM-DD.log`. `MIZU_LOG=mizu=debug` adds detail. The log warns about
  `slow UI-thread work` (over 50 ms) and `slow page load` (over 8 s).
- **Second launch** forwards its URLs to the running instance of the same profile.
- **Cargo can't replace a running `mizu.exe`.** Close it, or build into another folder with `CARGO_TARGET_DIR`.
- **DevTools for Mizu's own interface:** in debug builds, right-click the toolbar or a panel → Inspect.
- **Handy URL hooks** for looking at a state without clicking: `mizu://settings/#q=cookies`,
  `mizu://newtab/#customize`, `mizu://newtab/#folder=<id>`, `mizu://welcome/#2`.

---

## Where things live

| You want to change… | Look in |
|---|---|
| tab order, pinning, groups, spaces, tree tabs | `crates/mizu-tabs` (pure logic, unit tested) |
| windows, layout, split view | `crates/mizu-browser/src/window.rs` |
| opening/closing/activating tabs, tab events | `crates/mizu-browser/src/tabs.rs` |
| commands from the toolbar and panels | `crates/mizu-browser/src/commands.rs` |
| data for Mizu's pages (`mizu://…`) | `crates/mizu-browser/src/internal.rs` |
| settings and their defaults | `crates/mizu-settings/src/schema.rs` |
| history, bookmarks, shortcuts tiles, import | `crates/mizu-storage` |
| content blocking | `crates/mizu-privacy` + `crates/mizu-browser/src/scripts.rs` (in-page parts) |
| anything WebView2 / COM | `crates/mizu-engine` (the only crate that touches it) |
| the toolbar, tab strip, sidebar | `assets/ui/chrome/` |
| menus and panels | `assets/ui/flyout/` |
| Mizu's pages | `assets/ui/pages/` |
| colours, fonts, translations, icons | `assets/ui/shared/` (`tokens.css`, `i18n.js`, `icons.js`) |

---

## Recipes

### Add a command (interface → browser)

1. Add a variant to `Command` in `crates/mizu-ui/src/protocol.rs` (serde: `{"cmd":"snake_case_name", …}`). Use
   `#[serde(default)]` for optional fields so older callers keep working.
2. If only the toolbar or panels may send it, list it in `Command::is_chrome_only`.
3. Handle it in `commands.rs` (`handle`) or, for page data, in `internal.rs` (`handle_data`).
4. Call it from JS: `mizu.send('name', {…})`, or `await mizu.request('name', {…})` when you need an answer.

### Add a page (`mizu://something`)

1. Add a variant to `InternalPage` in `crates/mizu-core/src/internal.rs` (host and title).
2. Create `assets/ui/pages/something.html` and `something.js`, and register both in `surface()` in
   `crates/mizu-ui/src/assets.rs`.
3. Use the shared files (`/_/tokens.css`, `/_/pages.css`, `/_/bridge.js`, `/_/i18n.js`, `/_/icons.js`).

### Add a setting

1. Add the field and its default to the right struct in `crates/mizu-settings/src/schema.rs`; clamp or reset bad
   values in `sanitize`.
2. Show it in `assets/ui/pages/settings.js` (`toggle`, `setting`, `seg`, `select` helpers); `save()` persists.
3. If running code needs it on the UI thread hot path, copy it into `hot::Hot` where settings are applied
   (`app.rs`).

### Add interface text

- Write it in English in the code.
- Add the Russian line to the dictionary in `assets/ui/shared/i18n.js`. Text with numbers or names goes into
  `PATTERNS` as a regular expression.
- Mark website data (titles, addresses) with `translate="no"`.
- `scripts/check-ui.mjs` refuses duplicate dictionary keys.

### Styles

Use the variables from `shared/tokens.css` (`--surface`, `--ink-2`, `--accent`, `--r-md` …), never fixed colours, so
every palette and the dark theme keep working. Never use `innerHTML` with data; build elements with `h()` from
`bridge.js`.

---

## Code standards

- `cargo fmt`; clippy with no warnings.
- `Result` and `?`. `unwrap()`/`expect()` only in tests or on real invariants, with a comment.
- Every `unsafe` block has a `// SAFETY:` comment.
- New dependencies need a reason in the PR: what it replaces, licence, maintenance.
- Keep files focused; split modules past ~600–700 lines.
- Occasionally remove dead code (`cargo clippy -- -W dead_code`) and unused dependencies (`cargo machete`).

---

## Before a commit

```bash
node scripts/check-ui.mjs
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo test --workspace
```

`check-ui.mjs` checks every interface script's syntax, duplicate translation keys, and global names that clash
between shared and page scripts (a clash leaves a page blank). CI runs all three on every push
(`.github/workflows/ci.yml`). Commit only when all three pass.

---

## Releasing

1. **Version** — set `X.Y.Z` in:
   - `Cargo.toml` (workspace);
   - `crates/mizu-browser/resources/mizu.rc` and `mizu.manifest`;
   - `crates/mizu-setup/resources/setup.rc` and `setup.manifest`;
   - the installer file name in `README.md` and `docs/ru/README.md`.
2. **Docs** — a section in `CHANGELOG.md` and `docs/ru/CHANGELOG.md`; the guide and the roadmap (both languages) if
   features changed.
3. **Check** — the three commands above and the [manual checklist](TESTING.md#release-checklist).
4. **Commit and push.**
5. **Build:**

   ```bash
   powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -Notes "Short release notes"
   ```

   `dist/` then holds `MizuSetup-X.Y.Z.exe`, `Mizu-X.Y.Z-portable-x64.zip`, `mizu-update.json` and
   `mizu-update.json.sig`.
6. **Try the installer:** silent install (`MizuSetup-X.Y.Z.exe /S`), start it with a test profile, then
   `uninstall.exe --uninstall /S`. Mind an existing Start-menu shortcut: the installer replaces it.
7. **Publish** a GitHub release `vX.Y.Z` with all four files. Installed copies find it through
   `releases/latest/download/mizu-update.json` — open that URL to confirm it shows the new version.
8. **After:** check CI is green; `cargo clean` if you need the space.

---

## The signing key

Update manifests are signed with an ed25519 key at `%USERPROFILE%\.onibi\mizu-release.key` (`-Key` overrides it).

- **Never commit it** (`*.key` is ignored by git). Keep a backup somewhere safe: if it is lost, installed copies can
  no longer verify updates.
- `mizu-sign keygen <path>` makes a new key and prints the public key, which goes into `RELEASE_PUBLIC_KEY` in
  `crates/mizu-update/src/lib.rs`.
