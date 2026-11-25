# Mizu user guide

Everything you can do in Mizu, grouped by task. Short on time? Read [Quick start](#quick-start) and keep
[How do I…?](#how-do-i) for later.

**Contents**

1. [Quick start](#quick-start)
2. [How do I…?](#how-do-i)
3. [Tabs](#tabs) — basics, sleeping tabs, side tabs, tree tabs, groups, spaces, split view, sessions, private windows
4. [Address bar](#address-bar) — search, suggestions, calculator, units, quick actions, find on page
5. [Bookmarks, new tab, history, downloads](#bookmarks-new-tab-history-downloads)
6. [Privacy and Mizu Shield](#privacy-and-mizu-shield)
7. [Passwords](#passwords)
8. [Music and video](#music-and-video)
9. [Scripts and extensions](#scripts-and-extensions)
10. [Make it yours](#make-it-yours) — look, language, zoom, speed, experimental features
11. [Updates](#updates)
12. [Keyboard shortcuts](#keyboard-shortcuts)
13. [Mizu's pages](#mizus-pages)
14. [Troubleshooting](#troubleshooting)

---

## Quick start

1. **Install** `MizuSetup-x.y.z.exe` from the [latest release](https://github.com/senzukisu/mizu/releases/latest). No
   administrator rights needed.
2. **Welcome tour.** On first start Mizu shows a short tour. There you can make Mizu your default browser and import
   from another browser. Both can be done later in **Settings → General**.
3. **The five things to know:**
   - **Ctrl+T** new tab, **Ctrl+W** close, **Ctrl+Shift+T** bring the last one back.
   - **Ctrl+L** jumps to the address bar. Type `>` there to see quick actions.
   - The **shield** in the address bar blocks ads. If a site breaks, click it and turn Shield off for that site.
   - The **lock** in the address bar opens the site's settings: cookies, permissions, "Keep me signed in".
   - **Menu (⋮) → Settings** has a search box: type what you are looking for.
4. Prefer tabs on the side? **Settings → Tabs → Tab layout → Side.**

---

## How do I…?

| I want to… | Where |
|---|---|
| bring over bookmarks and history from Chrome, Edge, Firefox… | Settings → General → **Import** ([more](#import-from-another-browser)) |
| make Mizu my default browser | Settings → General → **Default browser** |
| put tabs on the side | Settings → Tabs → **Tab layout → Side** ([more](#side-tabs-and-pinned-sites)) |
| keep work and personal tabs apart | Menu → **New space** ([more](#spaces)) |
| see two pages side by side | right-click a tab → **Open in split view** ([more](#split-view)) |
| free memory | right-click a tab → **Unload other tabs** ([more](#sleeping-and-unloaded-tabs)) |
| stop being asked about a site without HTTPS | tick **Remember for this site** on that page ([more](#secure-connections-https-first)) |
| block a site's annoying element | Settings → Mizu Shield → **My rules** ([more](#mizu-shield)) |
| fix a site Shield breaks | click the **shield** → turn it off for the site |
| sign out of one site / forget it | lock icon → **Site settings** → **Forget this site** |
| stay signed in to a site forever | lock icon → **Site settings** → **Keep me signed in** |
| control music in other tabs | the **music button** in the toolbar ([more](#media-panel)) |
| pop a video out | hover it → **PiP**, or **Alt+Shift+P** ([more](#picture-in-picture)) |
| open a video in VLC or mpv | right-click the video → **Open video in …** ([more](#open-in-a-media-player)) |
| change a keyboard shortcut | Settings → **Shortcuts** ([more](#keyboard-shortcuts)) |
| change colours | Settings → **Appearance** ([more](#make-it-yours)) |
| put folders on the new tab page | drag one tile onto another ([more](#new-tab-page)) |
| use Tampermonkey scripts | Menu → **Scripts & extensions** ([more](#scripts-and-extensions)) |
| find any setting | Settings → search box (or **Ctrl+F** in Settings) |

---

## Tabs

### Basics

| Action | How |
|---|---|
| New tab / close tab | **Ctrl+T** / **Ctrl+W** or middle-click the tab |
| Reopen a closed tab | **Ctrl+Shift+T** (the last 25 are remembered) |
| Next / previous tab | **Ctrl+Tab** / **Ctrl+Shift+Tab**; **Ctrl+1…8** by position, **Ctrl+9** the last |
| Reorder | drag the tab |
| Pin, duplicate, mute, close others | right-click the tab |
| Many tabs | the strip scrolls sideways (mouse wheel or touchpad); the active tab stays in view |

### Sleeping and unloaded tabs

- **Unload tab** (right-click a tab) closes its page to free memory; the tab stays and reloads when you open it.
  **Unload other tabs** does it for every other tab. Tabs playing sound are skipped.
- **Sleeping tabs** (Settings → Performance): background tabs you haven't used for a while are suspended automatically.
- **Efficiency mode** (Settings → Performance): idle tabs use less memory after two minutes.

### Side tabs and pinned sites

Settings → Tabs → **Tab layout → Side**. The top becomes one row: navigation, address bar, menu, window buttons.

- Drag the sidebar's edge to resize it; double-click the edge to reset the width. The arrow at the bottom collapses it
  to icons.
- **Pinned sites** sit at the top as a grid of icons. Drag a tab up onto the grid to pin it, down into the list to
  unpin it.
- A pinned site remembers its page. Browse elsewhere and its icon gets a dashed outline; right-click it for **Back to
  pinned page** or **Pin this page instead**.

### Tree tabs

With side tabs, a tab opened from a page (a link, Ctrl+click) sits **under** it, indented.

- The arrow next to a tab folds or unfolds the tabs opened from it.
- Right-click → **Close tab and the tabs opened from it** closes the whole branch.
- Closing a tab moves its children up a level.
- Turn it off: Settings → Tabs → **Tree tabs**.

### Tab groups

- Right-click a tab → **Add to new group** (or to an existing group).
- **Click** the group's chip to collapse or expand it; **right-click** it to name it, pick a colour, ungroup or close
  its tabs.
- Drag a tab between grouped tabs to add it, or out of the group to remove it.

### Spaces

Separate sets of tabs in one window — for example *Work* and *Music*. Only the current space's tabs are shown.

- **Create:** Menu → **New space**, or right-click a tab → **Move to a new space**.
- **Switch:** the buttons at the bottom of the side tabs, or the space button at the start of the tab strip (it
  appears once you have two spaces). Each space remembers its last tab.
- **Edit:** click the current space's button (or right-click any) to rename it, choose a colour and an icon, add or
  delete a space. Deleting a space moves its tabs to the previous one.
- **Move a tab:** right-click it → **Move to …**.
- "Close other tabs", Ctrl+Tab and Ctrl+1…9 work within the current space.

### Split view

Two tabs side by side in one window.

- Right-click a tab → **Open in split view** (puts it next to the current tab), or right-click the current tab →
  **Split with a new tab**.
- Drag the divider to resize. Click into a half to make it the active tab.
- Right-click either tab → **Swap sides** or **Close split view**.

### Sessions

- Mizu reopens your tabs on start (Settings → General → **On startup**). Restored tabs load when you click them.
- With **Continue where I left off**, sites also stay signed in: Mizu keeps their session cookies, encrypted with your
  Windows account.
- After a crash Mizu offers to restore the session.
- Groups, spaces, split view and tree tabs are all restored.

### Private windows

**Ctrl+Shift+N.** Nothing goes to history, cookies are separate from normal windows, and everything is erased when
the last private window closes. Extensions don't run there.

---

## Address bar

| Type… | You get |
|---|---|
| an address or words | the page, or a search; **Enter** opens the first suggestion |
| part of an open tab's title | **Switch to this tab** (also in other windows) |
| `@g rust`, `@w water`, `@gh mizu` | one search with Google, Wikipedia, GitHub (add your own keywords in Settings → Search) |
| `2+2*3`, `(1,5+2)^2`, `15% * 80` | the answer; **Enter** copies it |
| `10 km in mi`, `70 f to c`, `1,5 gb in mb` | the converted value; **Enter** copies it |
| `>` | quick actions: clear this site's data, mute all, unload or close other tabs, new group, private window, open Mizu's pages… Type words to filter |

- **Suggestions** come from your history, bookmarks and Mizu's pages, computed on your PC. With **Search
  suggestions** on (Settings → Search), your search engine suggests too — that sends what you type to it, never in
  private windows.
- **Default search engine:** Settings → Search, or the letter button on the new tab page.
- **Find on page:** **Ctrl+F**. **Enter / Shift+Enter** (or **F3 / Shift+F3**) next / previous, **Aa** matches case,
  **Esc** closes.
- Icons on the right: the **key** (saved passwords for this site), the **star** (bookmark), the **shield**
  (blocked count).

---

## Bookmarks, new tab, history, downloads

### Import from another browser

Settings → General → **Import** (or `mizu://import/`). Mizu lists the profiles it finds: Chrome, Edge, Brave,
Vivaldi, Chromium, Opera, Opera GX and Firefox. Pick what to bring: bookmarks, history, search engines.

- Imported bookmarks go into a folder **Imported from …** in Other bookmarks; the other browser's bookmarks bar becomes
  a subfolder there. Tick **Also put its bar on Mizu's bookmarks bar** if you want those on the bar.
- Passwords: export a CSV from the other browser, then import it in Mizu Passwords.

### Bookmarks

- **Ctrl+D** or the star saves the page to **Other bookmarks**. Choose another folder in the panel and new bookmarks
  go there from then on.
- **Ctrl+Shift+O** opens the manager: search, drag to reorder, folders.
- The **bookmarks bar** shows only what you put on it, and appears with its first bookmark. **Ctrl+Shift+B** shows or
  hides it. **Other bookmarks** sits at its right end.
- Right-click a bookmark on the bar to open it in a new tab, window or private window, copy its link, or delete it.

### New tab page

- **Tiles** are their own list, separate from bookmarks. **+** adds one, the pencil edits, drag to reorder.
- **Folders:** drop a tile on the **middle** of another tile to make a folder (on a folder's middle to add to it; on a
  tile's edge to reorder). Click a folder to rename it, add or reorder its tiles, or take one out. Removing a folder
  keeps its tiles.
- **Customize** (bottom right): 13 backgrounds or your own picture with a darkening slider; clock and greeting (with
  your name); tile size; the Shield counter; widgets — **Now playing**, recently closed tabs, notes.

### History

**Ctrl+H.** Search, delete single entries, or clear the last hour, day, week, month or everything. Settings → Privacy
→ **Clear when Mizu closes** can do it automatically.

### Downloads

**Ctrl+J.** Each download shows speed and time left and can be paused, resumed or retried. Finished files: click to
open — PDFs, pictures, video, music and text open in a Mizu tab, other files in their Windows app; **Open in Windows
app** and **Open with…** pick otherwise. Settings → Downloads chooses the folder or makes Mizu ask every time.

---

## Privacy and Mizu Shield

### What Mizu sends

Mizu has **no telemetry**. On its own it only contacts: filter list servers, script update addresses, the update
manifest on GitHub, and — only if you turn them on — your search engine's suggestions. Your data lives in
`%LOCALAPPDATA%\Onibi Lab\Mizu\User Data`.

### Mizu Shield

Blocks ads, trackers and known malicious sites with uBlock Origin–compatible filter lists.

- The **shield** in the address bar shows how many requests were blocked. Click it to turn Shield off for the site.
- Settings → **Mizu Shield**:
  - **Filter lists:** turn lists on or off (Annoyances and EasyList Cookie are off by default) or add any list by its
    HTTPS address. Lists update by themselves; **Update lists now** forces it.
  - **My rules:** your own rules in uBlock syntax, e.g. `example.com##.cookie-banner` or
    `||ads.example^`.
- What Shield understands: network rules, `$redirect`, `$removeparam`, scriptlets, and element hiding — per-site
  rules, generic rules (`##.ad-banner` on every site) and procedural rules (`:has-text()`, `:upward()`, `:xpath()`,
  `:remove()`, `:style()` and more), including content a page adds later.
- `mizu://shield` is the dashboard: blocked today, this month and in total, a 30-day timeline, and the sites where
  the most was blocked. The numbers never leave your PC.

### Site settings

Click the **lock** → **Site settings** (or `mizu://site/?host=example.com`):

- Shield on or off for the site;
- its cookies, **Keep me signed in** (never cleaned up), **Forget this site** (cookies, storage, caches);
- permissions — camera, microphone, location, notifications…: Allow / Block / Ask;
- the zoom Mizu remembers for it, with **Reset**;
- your last visit.

### Secure connections (HTTPS-First)

When you open an `http://` address, Mizu goes to its secure `https://` version. If the site has none, you see **This
site has no secure version** with **Continue to the site (not secure)** — tick **Remember for this site** to not be
asked again. Routers, printers and other home-network addresses are left alone. Settings → Privacy → **Secure
connections** turns it off and lists the exceptions; the site's page (lock icon) has **Use HTTPS again**.

### Cookies

- **Third-party cookies** are blocked (Settings → Privacy; needs a restart). Turn this off only if an
  embedded sign-in stops working.
- **Tracking:** Settings → Privacy sets the tracking prevention level and **Global Privacy Control**.
- **Cookie cleanup:** forget sites you haven't visited for 7, 30 or 90 days, once a day. "Keep me signed in" sites are
  kept.
- **Clear browsing data:** **Ctrl+Shift+Delete**.

### Links to apps

When a site wants to open Steam, Telegram, Zoom, your mail app or another program, Mizu asks first. **Always allow**
remembers the choice for that site; Settings → Privacy → **Opening apps from sites** lists and removes them.

### Crash reports

If Mizu crashes, a short report is saved **on your PC only**. About Mizu shows how many there are and opens the
folder; attach one to a GitHub issue if you like.

More detail: [SECURITY](SECURITY.md).

---

## Passwords

- **Save:** after you sign in, Mizu asks whether to save the login. **Never for this site** stops it asking there.
- **Fill:** click the **key** in the address bar and pick a login.
- **Manage:** Settings → Passwords & security → **Open password manager** (or `mizu://passwords`): search, show, copy, edit.
- **Import / export:** CSV from Chrome, Edge, Firefox, Opera, Brave, Vivaldi or Bitwarden. An export is plain text —
  delete it after use.
- Passwords are encrypted with your Windows account (DPAPI) and never leave your PC.

---

## Music and video

### Media panel

While a tab plays sound or video, a **music button** appears in the toolbar. Its panel shows every playing tab with
the track and cover, previous / play-pause / next, a volume slider and **Go to tab**.

- The volume you set is remembered for that site (Settings → Media lists and resets them).
- **One sound at a time** (Settings → Media): when a tab starts playing, the others pause.
- Media keys on your keyboard work while Mizu is focused. Also: **Ctrl+Alt+Space** play/pause,
  **Ctrl+Alt+Right / Left** next / previous.
- The new tab page can show **Now playing** (Customize → Widgets).

### Picture in picture

Hover a video and press **PiP**, or press **Alt+Shift+P**. The floating window has previous/next video and 10-second
skip buttons. Settings → Media: button on/off, position, compact button.

### Open in a media player

With VLC, mpv, PotPlayer, MPC-HC or MPC-BE installed: right-click a video → **Open video in …**, or a link to a video
or song → **Open link in …**. YouTube, Twitch, Vimeo and SoundCloud pages are passed as pages, for players that can
open them. Settings → Media → **Media player** picks the player or turns it off.

### Codecs

`mizu://media` shows which formats play and whether the GPU decodes them. Details: [CODECS](CODECS.md).

---

## Scripts and extensions

### Mizu Scripts (userscripts)

Tampermonkey/Greasemonkey format. Menu → **Scripts & extensions** (`mizu://scripts`).

- **Install:** click a `.user.js` link on Greasy Fork or OpenUserJS, paste links (GitHub, GitLab, Gist, Codeberg…;
  several at once), or add files. A preview shows what the script does before you confirm.
- **Where it runs** (globe button): **Only sites I choose** or **Never run on**. `youtube.com` covers the whole site;
  `github.com/user` one section. Reload open pages afterwards.
- **Updates:** checked once a day; **Check for updates** checks now.

### Extensions

- On the **Chrome Web Store**, press **Add to Mizu** on an extension's page. Or install a `.crx`/`.zip` file or an
  unpacked folder (Scripts & extensions → Extensions).
- Chromium **Manifest V3** extensions only; Firefox add-ons are not supported. Extensions have no toolbar buttons
  yet: their pages open in a tab. They don't run in private windows.

---

## Make it yours

- **Settings → Appearance:** light, dark or system theme; palettes and any accent colour (optionally tinting the
  window); density; Soft, Round or Crisp style; Mica translucency (Windows 11); interface font; animations; toolbar
  buttons; tab layout.
- **Zoom:** **Ctrl+plus / minus**, **Ctrl+wheel**, **Ctrl+0** resets. Mizu remembers the zoom **per site**; reset it
  in the lock icon → Site settings. The default zoom is in Settings → Appearance.
- **Language:** Settings → General → Language: Automatic (follows Windows), English or Russian. Restart to apply
  everywhere.
- **Settings that need a restart** show a **Restart Mizu** button; your tabs come back.
- **Page menus:** right-click menus on web pages are drawn in Mizu's style; Settings → Appearance → **Mizu-style page
  menus** brings back the engine's own menu.
- **Speed** (Settings → Performance): **Connect while I type** opens the connection to the site you are typing before
  you press Enter; **Load pinned tabs after start** gets pinned sites ready in the background. The loading bar under
  the address bar follows the page's real progress.

### Experimental features

Settings → **Experimental**: features being tried out — they may change or go away, and turning one off undoes it.

- **Dark mode for every site** — with the dark theme, the engine darkens light pages (restart).
- **Thin scrollbars** — slim, Windows 11–style scrollbars (restart).
- **Parallel downloading** — big files over several connections (restart).
- **Double-click closes a tab** and **Mouse wheel switches tabs** (with side tabs, hold Shift).

---

## Updates

- **Settings → General → Updates → Update Mizu automatically:** Mizu checks shortly after start and every few hours,
  downloads and verifies the update, then asks **Update now / Later**. "Update now" restarts Mizu with your tabs.
- **Check for updates** checks right away.
- **What's new:** after each update a page shows what the new version brings (once, in Mizu's language). Turn it
  off there, in Settings → General → Updates, or in the welcome tour; `mizu://whatsnew` opens it any time.
- **Update channel:** Stable or **Beta** (gets new versions earlier, for testing).
- **If an update fails to start,** Mizu goes back to the previous version by itself and tells you; that version is not
  offered again.
- The **portable** copy doesn't update itself; it tells you when a new version is out.
- The web engine (WebView2) is updated by Windows.

---

## Keyboard shortcuts

Change any of the actions in **Settings → Shortcuts**: click the pencil and press new keys (Esc cancels). Keys taken
by another action move to the new one. The arrow resets one action; **Reset all shortcuts** resets everything.

| Tabs and windows | |
|---|---|
| New tab | Ctrl+T |
| Close tab | Ctrl+W, Ctrl+F4 |
| Reopen closed tab | Ctrl+Shift+T |
| Next / previous tab | Ctrl+Tab, Ctrl+PgDn / Ctrl+Shift+Tab, Ctrl+PgUp |
| Tab 1…8 / last tab | Ctrl+1…8 / Ctrl+9 |
| New window | Ctrl+N |
| New private window | Ctrl+Shift+N, Ctrl+Shift+P |
| Full screen | F11 |

| Pages | |
|---|---|
| Address bar | Ctrl+L, Alt+D, F6, Ctrl+K, Ctrl+E |
| Back / forward | Alt+Left / Alt+Right |
| Home page | Alt+Home |
| Reload / without cache | F5, Ctrl+R / Ctrl+F5, Shift+F5, Ctrl+Shift+R |
| Find on page / next / previous | Ctrl+F / F3, Ctrl+G / Shift+F3, Ctrl+Shift+G |
| Zoom in / out / reset | Ctrl+plus / Ctrl+minus / Ctrl+0 |
| Print | Ctrl+P |
| Picture in picture | Alt+Shift+P |

| Mizu | |
|---|---|
| Bookmark this page | Ctrl+D |
| Show or hide bookmarks bar | Ctrl+Shift+B |
| Bookmarks / History / Downloads | Ctrl+Shift+O / Ctrl+H / Ctrl+J |
| Clear browsing data | Ctrl+Shift+Delete |
| Play or pause music / next / previous track | Ctrl+Alt+Space / Ctrl+Alt+Right / Ctrl+Alt+Left |
| Task manager | Shift+Esc |
| Developer tools | F12 (when enabled in Settings → Advanced) |

Ctrl+F, Ctrl+P, zoom, F12 and Ctrl+1…9 are handled by the web engine and can't be changed.

---

## Mizu's pages

Type these in the address bar:

| Page | What it is |
|---|---|
| `mizu://settings` | Settings (`#q=word` opens with a search) |
| `mizu://history` · `mizu://bookmarks` · `mizu://downloads` | History, bookmarks, downloads |
| `mizu://passwords` | Password manager |
| `mizu://scripts` | Userscripts and extensions |
| `mizu://shield` | Shield statistics |
| `mizu://site/?host=example.com` | One site's settings |
| `mizu://import` | Import from another browser |
| `mizu://media` | Codecs and hardware decoding |
| `mizu://about` | Version, engine, updates, crash reports |
| `mizu://welcome` | The welcome tour again |
| `mizu://whatsnew` | What the latest versions brought |

---

## Troubleshooting

| Problem | What to do |
|---|---|
| A site looks broken | Click the shield and turn Shield off for the site. If that helps, report the site to the filter list's maintainers. |
| A site keeps signing me out | Lock → Site settings → **Keep me signed in**. If it uses an embedded sign-in, try allowing third-party cookies (Settings → Privacy). |
| Video stutters | Open `mizu://media`. If the codec isn't "Hardware", update the GPU driver; for HEVC install "HEVC Video Extensions". |
| Slow with many tabs | Right-click a tab → **Unload other tabs**; keep sleeping tabs on (Settings → Performance). |
| A shortcut does nothing | Settings → Shortcuts: the keys may have moved to another action. **Reset all** if unsure. |
| No search suggestions | Settings → Search → **Search suggestions** must be on; never in private windows; some engines have none. |
| The bookmarks bar shows things I didn't add | Right-click them → Delete, or hide the bar with Ctrl+Shift+B. |
| "Open video in …" is missing | No supported player was found, or it's off: Settings → Media → Media player. |
| "WebView2 Runtime missing" | Install the Microsoft Edge WebView2 Runtime from Microsoft, then start Mizu again. |
| Something else | Logs are in `%LOCALAPPDATA%\Onibi Lab\Mizu\User Data\logs`. Attach the latest one to an [issue](https://github.com/senzukisu/mizu/issues). |
