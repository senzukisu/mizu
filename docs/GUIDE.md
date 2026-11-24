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
  - **Pinned tabs** sit at the top as a grid of icons. Drag a tab up onto the grid to pin it, and down into the list
    to unpin it.
  - A pinned tab remembers its page. When you browse to another site its icon gets a dashed outline; right-click it
    for **Back to pinned page** or **Pin this page instead**.
- **Sessions.** By default Mizu reopens your tabs when it starts. Restored tabs load only when you click them.
  After a crash, Mizu offers to restore the session. With **Continue where I left off**, sites also stay signed in:
  Mizu keeps their session cookies, encrypted with your Windows account.
- **Private windows.** Ctrl+Shift+N opens one.
  - Nothing goes to history.
  - Cookies are separate from normal windows, and everything is erased when the last private window closes.

- **Tree tabs.** With tabs on the side, a tab opened from a page sits under it. The arrow folds a branch;
  right-click → **Close tab and the tabs opened from it** closes the whole branch. Settings → Tabs → **Tree tabs**
  turns it off.
- **Spaces.** Menu → **New space** makes a separate set of tabs in the same window (say Work and Music). Switch
  with the buttons at the bottom of the side tabs or the space button at the start of the tab strip; click the
  current space's button (or right-click any) to rename it, pick a colour and an icon, add or delete a space.
  Right-click a tab → **Move to…** sends it to another space. Deleting a space moves its tabs to the one before.
- **Split view.** Right-click a tab → **Open in split view** to see it next to the current tab (or **Split with a
  new tab**). Drag the divider between them; the tab menu has **Swap sides** and **Close split view**.
- **Tab groups.** Right-click a tab → **Add to new group** (or to an existing one). Click the group's chip to
  collapse or expand it; right-click the chip to name it, pick a colour, ungroup or close its tabs. Drag a tab
  between grouped tabs to add it, or out of the group to remove it.

## Address bar and search

- Type an address or a search; Enter opens the first suggestion.
- **Suggestions** come from your history, bookmarks and Mizu's pages (computed on your PC). With **Search
  suggestions** on, they also come from your search engine. That sends what you type to the engine, never in
  private windows. You can turn either off in Settings → Search.
- **Search engines.** Change the default in Settings → Search or on the new tab page.
  - A keyword searches once with another engine: `@g rust` (Google), `@w water` (Wikipedia), `@gh mizu` (GitHub).
  - You can add your own engines with their own keywords.
- **Open tabs.** Type part of a tab's title or address to get **Switch to this tab**, also for tabs in other
  windows.
- **Calculator.** Type `2+2*3`, `(1,5+2)^2` or `15% * 80`: the answer is the first suggestion and Enter copies it.
- **Units.** `10 km in mi`, `70 f to c`, `2 kg to lb`, `1,5 gb in mb`: the answer comes first, Enter copies it.
- **Quick actions.** Type `>` to see them: clear this site's cookies and data, mute or unmute all tabs, unload or
  close other tabs, new group, private window, open Mizu's pages and more. Typing words narrows the list.
- **Find on page.** Ctrl+F opens a bar under the toolbar. Enter / Shift+Enter (or F3 / Shift+F3) go to the next or
  previous match, **Aa** matches case, Esc closes.
- The **key** icon appears when you have a saved password for the site, and the **star** bookmarks the page.
- The **lock** shows the connection, the site's permissions and its cookies. **Clear site data** removes the site's
  cookies and storage (you are signed out of it) and reloads the page.

## Bookmarks, history and downloads

- **Import from another browser.** Settings → General → Import, the import button in Bookmarks, or `mizu://import/`.
  Pick a browser profile found on this PC and choose bookmarks, history and search engines. Passwords come from the
  other browser's CSV export, imported in Mizu Passwords.

- **Bookmarks.**
  - Ctrl+D or the star saves a page to **Other bookmarks**. Pick another folder in the "Bookmark added" panel and new
    bookmarks go there from then on. Ctrl+Shift+O opens the manager, where you can drag to reorder and create folders.
  - The **bookmarks bar** shows only what you put on it and appears as soon as it has a bookmark (Ctrl+Shift+B shows
    or hides it). **Other bookmarks** sits at its right end.
  - Right-click a bookmark on the bar to open it in a new tab, window or private window, copy its link or delete it.
- **New tab page.** Its tiles are a list of their own, separate from bookmarks. "+" adds one, the pencil edits it, and
  you can drag tiles to reorder them. Drop a tile on the middle of another to make a **folder**; click a folder to
  rename it, add shortcuts or take one out. **Customize** (bottom right) picks one of 13 backgrounds or your own picture (with a
  darkening slider) and shows or hides the clock, the tiles and the Shield counter.
- **History.** Ctrl+H. Search, delete single entries, or clear the last hour, day, week, month or everything.
  Settings → Privacy can also clear data when Mizu closes.
- **Downloads.** Ctrl+J.
  - Each download shows its speed and time left, and can be paused, resumed or retried.
  - Files are grouped by day, with a badge for their type; click a finished file to open it, or **Open with…** to
    pick the program. The folder button opens
    your downloads folder.
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

## Music and video

- While a tab plays sound or video, a **music button** appears in the toolbar. Its panel shows every playing tab
  with the track and cover, previous / play-pause / next, a volume slider and **Go to tab**.
- The volume you set is remembered for that site. Settings → Media lists the sites and can reset them.
- **One sound at a time** (Settings → Media) pauses other tabs when one starts playing.
- The keyboard's media keys work while Mizu is focused. Ctrl+Alt+Space plays or pauses, Ctrl+Alt+Right / Left switch
  tracks; change them in Settings → Shortcuts.
- The new tab page can show **Now playing** (Customize → Widgets).

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
- **Zoom per site.** Zoom a site in or out and Mizu keeps that zoom for it; lock icon → **Site settings** → **Reset**
  returns it to the default.
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
- **Site settings.** Click the lock in the address bar → **Site settings**: Shield for that site, its cookies,
  **Keep me signed in**, **Forget this site** (cookies and stored data), permissions and your last visit.
- **Cookie cleanup.** Settings → Privacy → Cookie cleanup forgets sites you haven't visited for 7, 30 or 90 days
  (once a day). Sites with "Keep me signed in" are kept.
- **Crash reports** stay on your PC (About Mizu → Privacy shows them); attach one to a GitHub issue if you like.
- **Third-party cookies** are blocked (Settings → Privacy, needs a restart). Turn it off only if an embedded sign-in
  stops working.
- **Links to apps.** When a site wants to open Steam, Telegram, Zoom, your mail app or another program, Mizu asks
  first. **Always allow** remembers the site; Settings → Privacy → Opening apps from sites lists and removes them.
- **Settings search.** In Settings, type in the search box (or press Ctrl+F) to find any option. Changes that need a
  restart show a **Restart Mizu** button; your tabs come back.
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
