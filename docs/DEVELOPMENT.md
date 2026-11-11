# Developing Mizu

## Prerequisites

- Windows 10/11 x64
- Rust stable ≥ 1.85 with the `x86_64-pc-windows-msvc` toolchain
- Visual Studio Build Tools: "Desktop development with C++" (MSVC, Windows SDK; `rc.exe` is used for the icon/manifest)
- Microsoft Edge WebView2 Runtime (preinstalled on Windows 11)
- Optional: Node.js, only for `node --check` on the UI scripts

## Build and run

```bash
cargo build
```

```bash
cargo run
```

Use a throwaway profile while developing so your real data is never touched:

```bash
MIZU_USER_DATA="$TEMP/mizu-dev" MIZU_LOG=mizu=debug cargo run
```

While a copy of Mizu runs from `target\debug`, cargo can't replace it. Build into another folder to test side by side:

```bash
CARGO_TARGET_DIR=target/vh cargo run
```

The log warns about slow UI-thread work (`slow UI-thread work`, over 50 ms) and slow pages (`slow page load`, over 8 s).

Logs are written to `<user data>/logs/mizu.YYYY-MM-DD.log`. A second launch forwards its URLs to the running
instance, which is handy for testing: `cargo run -- https://example.com`.

Release build (thin LTO, stripped):

```bash
cargo build --release
```

## Project layout

See [ARCHITECTURE.md](ARCHITECTURE.md). In short:

- Pure logic goes in the platform-independent crates and gets unit tests.
- Only `mizu-engine` touches COM, and every `unsafe` block there has a `// SAFETY:` comment.
- UI lives in `assets/ui` and is compiled into the binary by `mizu-ui::assets` (add new files to that map).

## UI development

- `shared/tokens.css` holds the design tokens. Use the variables (`--surface`, `--ink-2`, `--accent`, `--r-md` …) and
  never hard-code colours, so every palette and the dark theme keep working.
- `shared/bridge.js` provides `mizu.send`, `mizu.request`, `mizu.on` and the `h()` DOM helper. Never use `innerHTML`
  with data.
- A new command needs a variant in `mizu-ui::protocol::Command` and a handler in `mizu-browser::commands` or
  `mizu-browser::internal`. Decide whether it is chrome-only (`Command::is_chrome_only`).
- Developer tools work in debug builds for the chrome and flyout views as well (right-click → Inspect).
- **Translations.**
  - Write UI text in English.
  - Add the Russian string to the dictionary in `shared/i18n.js`, or a pattern for text that contains numbers or names.
  - Mark website data with `translate="no"` so it is never translated.

## Code standards

- `cargo fmt`, and `cargo clippy --workspace` with no warnings.
- Prefer `Result` and `?`. `unwrap()`/`expect()` only in tests or on true invariants, with a comment.
- No new dependency without a reason in the PR description (what it replaces, its licence, its maintenance status).
- Keep files focused. Split modules beyond roughly 600–700 lines.
- Periodic audit: remove dead code (`cargo clippy -- -W dead_code`) and unused dependencies (`cargo machete` or
  `cargo udeps`).

## Releasing

1. Update the version in:
   - the root `Cargo.toml`;
   - `crates/mizu-browser/resources/mizu.rc`;
   - `crates/mizu-setup/resources/setup.rc` and `setup.manifest`;
   - `CHANGELOG.md` and `docs/ru/CHANGELOG.md`.
2. Run `cargo test --workspace` and the manual checklist in [TESTING.md](TESTING.md).
3. Build everything:

   ```bash
   powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -Notes "Short release notes"
   ```

   `dist/` then contains `MizuSetup-X.Y.Z.exe`, `Mizu-X.Y.Z-portable-x64.zip`, `mizu-update.json` and
   `mizu-update.json.sig`.
4. Tag `vX.Y.Z` and create a GitHub release with all four files. Installed copies find the update through
   `releases/latest/download/mizu-update.json`.

### The signing key

Update manifests are signed with an ed25519 key at `%USERPROFILE%\.onibi\mizu-release.key` (override it with `-Key`).
- It must **never** be committed; `*.key` is ignored by git.
- `mizu-sign keygen <path>` creates a key and prints its public key, which goes into `RELEASE_PUBLIC_KEY` in
  `crates/mizu-update/src/lib.rs`.
- If the key changes, older versions can no longer verify updates, so keep a backup in a safe place.
