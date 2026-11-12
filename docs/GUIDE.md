# Mizu user guide

How to use Mizu day to day. For installation see the [README](../README.md); for how Mizu is built see
[ARCHITECTURE](ARCHITECTURE.md).

- [Tabs and windows](#tabs-and-windows)
- [Address bar and search](#address-bar-and-search)
- [Bookmarks, history and downloads](#bookmarks-history-and-downloads)
- [Mizu Shield](#mizu-shield)
- [Passwords](#passwords)
- [Mizu Scripts](#mizu-scripts)
- [Extensions](#extensions)
- [Video and picture in picture](#video-and-picture-in-picture)
- [Keyboard shortcuts](#keyboard-shortcuts)
- [Appearance and language](#appearance-and-language)
- [Updates](#updates)
- [Privacy](#privacy)
- [Troubleshooting](#troubleshooting)

## Tabs and windows

- **Open, close, reopen.**
  - Ctrl+T opens a tab; Ctrl+W or a middle click closes it.
  - Ctrl+Shift+T brings back the last closed tab; Mizu remembers the last 25.
- **Arrange.**
  - Drag tabs to reorder them.
  - Right-click a tab to pin, duplicate, mute or close it, or to close the tabs to its right.
- **Many tabs.** When tabs reach their minimum width, the strip scrolls sideways. Use the mouse wheel or touchpad over
  it, and the active tab always scrolls into view.
- **Unload a tab.** Right-click a tab you aren't using and choose **Unload tab**, or **Unload other tabs**.
  - Its page closes and frees memory, but the tab stays in the strip.
  - It reloads when you open it.
  - Tabs that are playing sound are skipped.
- **Sleeping tabs.** Background tabs are also suspended automatically after a while (Settings → Tabs).
  Efficiency mode lowers the memory of tabs that have been idle for two minutes (Settings → Performance).
- **Vertical tabs.** Settings → Appearance → Tab layout → Side. The top becomes a single row: navigation, the address
  bar, the menu and the window buttons.
  - Drag the sidebar's edge to resize it; double-click the edge to reset its width.
  - The arrow at the bottom collapses it to icons.
- **Sessions.** By default Mizu reopens your tabs when it starts. Restored tabs load only when you click them.
  After a crash, Mizu offers to restore the session.
- **Private windows.** Ctrl+Shift+N opens one.
  - Nothing goes to history.
  - Cookies are separate from normal windows, and everything is erased when the last private window closes.

## Address bar and search

- Type an address or a search; Enter opens the first suggestion.
- **Suggestions** come from your history, bookmarks and Mizu's pages (computed on your PC). With **Search
  suggestions** on, they also come from your search engine. That sends what you type to the engine, never in
  private windows. You can turn either off in Settings → Search.
- **Search engines.** Change the default in Settings → Search or on the new tab page.
  - A keyword searches once with another engine: `@g rust` (Google), `@w water` (Wikipedia), `@gh mizu` (GitHub).
  - You can add your own engines with their own keywords.
- The **key** icon appears when you have a saved password for the site, and the **star** bookmarks the page.

## Bookmarks, history and downloads

- **Bookmarks.**
  - Ctrl+D or the star saves a page; Ctrl+Shift+O opens the manager, where you can drag to reorder and create folders.
  - The **bookmarks bar** appears as soon as it has a bookmark (Ctrl+Shift+B shows or hides it).
  - Right-click a bookmark on the bar to open it in a new tab, window or private window, copy its link or delete it.
- **New tab page.** Bookmarks from the bar appear as tiles, and "+" adds one.
- **History.** Ctrl+H. Search, delete single entries, or clear the last hour, day, week, month or everything.
  Settings → Privacy can also clear data when Mizu closes.
- **Downloads.** Ctrl+J.
  - Each download shows its speed and time left, and can be paused, resumed or retried.
  - Settings → Downloads chooses the folder or makes Mizu ask every time.

## Mizu Shield

Mizu Shield blocks ads, trackers and known malicious sites using uBlock Origin-compatible filter lists.

- The **shield** in the address bar shows how many requests were blocked on the page. Click it to turn Shield off
  for that site, for example if a site breaks.
- **Settings → Privacy & Shield:**
  - turn lists on or off (Annoyances and EasyList Cookie are off by default), or add any list by its HTTPS address;
  - write your own rules in uBlock syntax;
  - set the tracking prevention level and Global Privacy Control.
- **`mizu://shield`** shows statistics.
- Lists update automatically. If a list shows "Update failed", it retries on its own and has a mirror for the uBlock
  lists. You can also press "Update lists now".

## Passwords

- **Saving.** After you sign in, Mizu asks whether to save the login and password. **Never for this site** stops it
  asking there.
- **Filling.** Click the **key** in the address bar and choose a login.
- **Managing.** Settings → Security → **Open password manager**, or `mizu://passwords`. There you can search, show,
  copy and edit logins.
- **Import and export.** Import CSV files exported from Chrome, Edge, Firefox, Opera, Brave, Vivaldi or Bitwarden.
  Export writes a CSV in plain text, so delete it after use.
- Passwords are encrypted with your Windows account (DPAPI) and never leave your PC.

## Mizu Scripts

Userscripts in the Tampermonkey/Greasemonkey format. Open them from Menu → Scripts & extensions (`mizu://scripts`).

- **Install** a script in any of these ways:
  - click a `.user.js` link on Greasy Fork or OpenUserJS;
  - paste links (GitHub, GitLab, Gist, Codeberg and more), several at once;
  - add files.

  A preview shows what each script does before you confirm.
- **Where a script runs.** The globe button next to a script opens **Where it runs**:
  - **Only sites I choose** limits the script to the sites you list;
  - **Never run on** excludes sites.

  `youtube.com` covers all its pages and subdomains, and `github.com/user` covers one section. Reload open pages
  after changing this.
- **Updates.** Scripts with an update address are checked once a day. "Check for updates" checks immediately.

## Extensions

- Open an extension's page on the **Chrome Web Store** and press **Add to Mizu**. You can also install from a `.crx`
  or `.zip` file, or from an unpacked folder (Scripts & extensions → Extensions).
- Extensions don't run in private windows, and their options open in a tab.
- Mizu runs on Microsoft Edge WebView2, so it supports **Chromium (Manifest V3)** extensions. Firefox add-ons are not
  supported.

## Video and picture in picture

- **Picture in picture.** Hover a video and press **PiP**, or press **Alt+Shift+P** anywhere on the page.
- The floating window has **previous/next video** buttons (on YouTube they follow the player's playlist) and
  **10-second skips**.
- **Settings → Media:** the button on or off, its position, a compact icon-only button, and video switching.
- **Codecs.** `mizu://media` shows which formats play and whether the GPU decodes them. See [CODECS](CODECS.md).

## Keyboard shortcuts

- **Settings → Shortcuts:**
  - click the **pencil** next to an action and press the new keys (Esc cancels);
  - if those keys belonged to another action, they move to the new one;
  - the arrow resets one action, and **Reset all shortcuts** resets everything.
- Page shortcuts handled by the engine (Ctrl+F, Ctrl+P, zoom, F12, Ctrl+1…9) are listed for reference and can't be
  changed.

## Appearance and language

- **Settings → Appearance:**
  - light, dark or system theme;
  - five palettes and any accent colour;
  - density, soft or crisp style, and Mica translucency on Windows 11;
  - animations, default page zoom (Ctrl+0 resets a page), and tab layout.
- **Language.** Settings → General → Language: Automatic (follows Windows), English or Russian. A restart applies it
  everywhere.

## Updates

- With **Update Mizu automatically** on (Settings → General → Updates), Mizu checks for a new version shortly after
  it starts and every few hours.
  - It downloads and verifies the update, then asks **Update now / Later**.
  - "Update now" restarts Mizu, and your tabs come back.
- **Check for updates** checks immediately.
- The **portable** copy doesn't update itself; it tells you when a new version is out.
- The web engine is updated by Windows.

## Privacy

- Mizu sends no telemetry. History, bookmarks, passwords and settings stay in
  `%LOCALAPPDATA%\Onibi Lab\Mizu\User Data`.
- Network requests Mizu makes on its own:
  - filter lists;
  - userscript and update checks;
  - the update manifest on GitHub;
  - search suggestions, if you turn them on.
- More detail: [SECURITY](SECURITY.md).

## Troubleshooting

| Problem | What to do |
|---|---|
| A site looks broken | Click the shield and turn Shield off for the site. If that fixes it, report the site to the filter list's maintainers. |
| Video stutters | Open `mizu://media`. If the codec isn't "Hardware", update your GPU driver. For HEVC install "HEVC Video Extensions". |
| Mizu feels slow with many tabs | Unload tabs you aren't using (right-click → Unload other tabs) and keep sleeping tabs on (Settings → Tabs). |
| A shortcut does nothing | Check Settings → Shortcuts: the keys may have moved to another action. Reset all if unsure. |
| No search suggestions | Settings → Search → Search suggestions must be on; they never appear in private windows. Some engines (Startpage, GitHub) have none. |
| "WebView2 Runtime missing" | Install the Microsoft Edge WebView2 Runtime from Microsoft's site, then start Mizu again. |
| Something else | `%LOCALAPPDATA%\Onibi Lab\Mizu\User Data\logs` has logs. Include the latest one in an [issue](https://github.com/senzukisu/mizu/issues). |
