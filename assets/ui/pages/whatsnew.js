// What's new: opens once after Mizu updates (Settings → General → Updates turns it off).
// mizu://whatsnew/?from=0.3.4 shows everything since that version.
'use strict';

const main = document.getElementById('main');
const REPO = 'https://github.com/senzukisu/mizu';

// Newest first. [icon, title, text, optional Mizu link]
const RELEASES = [
  { v: '0.4.0', name: 'Deep Water', items: [
    ['lock', 'HTTPS-First', 'Mizu opens the secure version of a site when you go to an http address. If a site has none, it asks before loading it without encryption — and can remember your answer.', 'mizu://settings/#privacy'],
    ['shield', 'A new Shield dashboard', 'How much was blocked today and this month, a 30-day timeline, and the sites where Shield stopped the most. Kept on this PC only.', 'mizu://shield/'],
    ['menu', 'New menus', 'The ⋮ menu has big buttons for what you open most, and right-click menus on pages now look like the rest of Mizu.'],
    ['upload', 'Updates you can watch', 'Settings and the toolbar show the new version downloading, with a progress bar.', 'mizu://settings/#general'],
    ['sparkle', 'The start of 0.4', 'Next in this series: fingerprinting defence and encrypted backups.'],
  ] },
  { v: '0.3.6', name: 'Prism', items: [
    ['sparkle', 'What’s new, after every update', 'This page opens once after Mizu updates, in the language you use Mizu in. Turn it off below, in Settings or in the welcome tour.', 'mizu://settings/#general'],
    ['download', 'Downloads open in Mizu', 'Click a finished PDF, picture, video or song to open it in a new tab. Other files still open in their Windows app.', 'mizu://settings/#downloads'],
    ['sun', 'Sites follow your theme', 'Light theme, light sites; dark theme, dark sites. Pages without a background of their own stay readable, and the experimental dark mode for every site only works with the dark theme.', 'mizu://settings/#appearance'],
    ['reload', 'Restart that always comes back', 'Restarting Mizu to apply a setting now waits for the engine to close, and tries once more by itself if it has to.'],
    ['gauge', 'Ready for 0.4', 'The next big step — HTTPS-First, fingerprinting defence and encrypted backups — is planned step by step in the roadmap.'],
  ] },
  { v: '0.3.5', items: [
    ['upload', 'Updates that undo themselves', 'If a new version doesn’t start, Mizu goes back to the previous one and tells you. A beta channel gets versions earlier.', 'mizu://settings/#general'],
    ['gauge', 'Faster loading', 'Mizu connects to a site while you type its address, loads pinned tabs in the background, and the loading bar shows real progress.', 'mizu://settings/#performance'],
    ['sliders', 'Clearer settings', 'Grouped into Browser, Protection and More, with Shield, Privacy and Passwords on their own pages.', 'mizu://settings/'],
    ['sparkle', 'Experimental', 'Dark mode for every site, thin scrollbars, parallel downloading and more to try.', 'mizu://settings/#labs'],
  ] },
  { v: '0.3.4', items: [
    ['shield', 'Shield hides more', 'Generic and procedural filters remove ad blocks everywhere, even ones a page adds later.', 'mizu://settings/#shield'],
    ['media', 'Open in your media player', 'Right-click a video → Open in VLC, mpv, PotPlayer or MPC.', 'mizu://settings/#media'],
  ] },
  { v: '0.3.3', items: [
    ['tabs', 'Tree tabs', 'With side tabs, a tab opened from a page sits under it. Fold a branch with its arrow.', 'mizu://settings/#tabs'],
    ['folder', 'Folders on the new tab page', 'Drop one tile on another to make a folder.'],
    ['zoom-in', 'Zoom per site', 'Mizu remembers the zoom you choose on each site.'],
  ] },
  { v: '0.3.2', items: [
    ['sparkle', 'Spaces', 'Separate sets of tabs — work, music, study — each with its own colour and icon.'],
    ['split', 'Split view', 'Two tabs side by side: right-click a tab → Open in split view.'],
  ] },
  { v: '0.3.1', items: [
    ['lock', 'Site settings', 'The lock icon opens a page per site: Shield, cookies, Keep me signed in, permissions.'],
    ['cookie', 'Cookie cleanup', 'Forget sites you haven’t visited for a while.', 'mizu://settings/#privacy'],
  ] },
  { v: '0.3.0', name: 'Currents', items: [
    ['import', 'Import from your old browser', 'Bookmarks, history and search engines from Chrome, Edge, Firefox and others.', 'mizu://import/'],
    ['folder', 'Tab groups and quick actions', 'Name and colour groups of tabs; type > in the address bar for quick actions.'],
  ] },
];

// a < b for versions like 0.3.10
function older(a, b) {
  const pa = String(a).split('.').map(Number);
  const pb = String(b).split('.').map(Number);
  for (let i = 0; i < 3; i++) {
    if ((pa[i] || 0) !== (pb[i] || 0)) return (pa[i] || 0) < (pb[i] || 0);
  }
  return false;
}

function openPage(url) {
  if (url) mizu.send('open_url', { url, disposition: 'current' });
}

function itemCard([ic, title, text, url]) {
  const card = h(url ? 'button' : 'div', { class: `wn-card${url ? ' link' : ''}` },
    h('div', { class: 'wn-icon' }, icon(ic, { size: 22 })),
    h('div', { class: 'wn-t', text: title }),
    h('div', { class: 'wn-d', text }),
    url ? h('div', { class: 'wn-go' }, h('span', { text: 'Open' }), icon('forward', { size: 14 })) : null);
  if (url) card.addEventListener('click', () => openPage(url));
  return card;
}

// Earlier versions as a timeline: a card per version with small tiles; the two most recent
// are shown, the rest behind "Show earlier versions".
function releaseCard(r) {
  return h('article', { class: 'wn-rel' },
    h('header', { class: 'wn-rel-head' },
      h('span', { class: 'wn-badge', translate: 'no', text: r.v }),
      r.name ? h('span', { class: 'wn-rel-name', translate: 'no', text: r.name }) : null),
    h('div', { class: 'wn-tiles' }, ...r.items.map(([ic, title, text, url]) => {
      const tile = h(url ? 'button' : 'div', { class: `wn-tile${url ? ' link' : ''}` },
        h('span', { class: 'wn-tile-icon' }, icon(ic, { size: 16 })),
        h('span', { class: 'wn-tile-txt' }, h('span', { class: 'wn-t', text: title }), h('span', { class: 'wn-d', text })));
      if (url) tile.addEventListener('click', () => openPage(url));
      return tile;
    })));
}

function timeline(releases, from) {
  const SHOWN = 2;
  const list = h('div', { class: 'wn-timeline' }, ...releases.slice(0, SHOWN).map(releaseCard));
  const rest = releases.slice(SHOWN);
  let more = null;
  if (rest.length) {
    more = h('button', { class: 'wn-more' }, icon('chevron-down', { size: 16 }), h('span', { text: i18n.tr(rest.length === 1 ? 'Show 1 earlier version' : `Show ${rest.length} earlier versions`) }));
    more.addEventListener('click', () => {
      more.remove();
      for (const r of rest) {
        const c = releaseCard(r);
        c.classList.add('wn-in');
        list.append(c);
      }
    });
  }
  return h('section', { class: 'wn-earlier' },
    h('h2', { text: from ? 'Also since your last version' : 'Earlier versions' }),
    list, more);
}

function render(settings, version) {
  const params = new URLSearchParams(location.search);
  const from = params.get('from') || '';
  const current = RELEASES.find((r) => r.v === version) || RELEASES[0];
  const earlier = RELEASES.filter((r) => r !== current && older(r.v, current.v) && (!from || older(from, r.v)));
  const ru = (document.documentElement.lang || '').startsWith('ru');
  const changelog = `${REPO}/blob/main/${ru ? 'docs/ru/' : ''}CHANGELOG.md`;

  const g = settings.general;
  const toggle = switchInput(g.whats_new !== false, (v) => {
    g.whats_new = v;
    mizu.request('set_settings', { settings }).then(() => showToast(v ? 'You’ll see this page after updates' : 'This page won’t open after updates')).catch((e) => showToast(e.message));
  }, { label: 'Show after every update' });

  const all = h('button', { class: 'btn' }, icon('file'), h('span', { text: 'Every change, in detail' }));
  all.addEventListener('click', () => mizu.send('open_url', { url: changelog, disposition: 'new_tab' }));
  const done = h('button', { class: 'btn primary', text: 'Start browsing' });
  done.addEventListener('click', () => openPage('mizu://newtab/'));

  main.replaceChildren(
    h('section', { class: 'wn-hero' },
      prismCanvas({ fps: 60 }),
      h('div', { class: 'wn-hero-text' },
        h('div', { class: 'wn-kicker', translate: 'no', text: `Mizu ${current.v}${current.name ? ` · ${current.name}` : ''}` }),
        h('h1', { text: 'What’s new' }),
        h('p', { text: from ? i18n.tr(`Since ${from}, here is what changed.`) : 'Here is what this update brings.' }))),
    h('div', { class: 'page wn' },
      h('div', { class: 'wn-grid' }, ...current.items.map(itemCard)),
      earlier.length ? timeline(earlier, from) : null,
      h('div', { class: 'wn-foot' },
        h('label', { class: 'wn-switch' }, toggle, h('span', { text: 'Show after every update' })),
        h('div', { class: 'grow' }),
        all, done)),
  );
}

bootPage(() => {
  Promise.all([mizu.request('get_settings'), mizu.request('update_state').catch(() => ({}))]).then(([p, u]) => {
    render(structuredClone(p.settings), u.version || RELEASES[0].v);
    // mizu://whatsnew/#earlier opens at the earlier versions.
    if (location.hash === '#earlier') requestAnimationFrame(() => document.querySelector('.wn-earlier')?.scrollIntoView({ block: 'start' }));
  });
});
