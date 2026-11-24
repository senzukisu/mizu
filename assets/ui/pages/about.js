// About Mizu.
'use strict';

const main = document.getElementById('main');
const REPO = 'https://github.com/senzukisu/mizu';

// What Mizu does, each tile opening the place where it lives.
const FEATURES = [
  ['shield-check', 'Mizu Shield', 'Ads and trackers blocked with uBlock Origin–style filter lists.', 'mizu://shield/'],
  ['cookie', 'Cookies under control', 'Third-party cookies blocked. See and clear any site\u2019s data from the lock icon.', 'mizu://settings/#privacy'],
  ['key', 'Passwords', 'Logins encrypted with your Windows account, filled from the address bar.', 'mizu://passwords/'],
  ['tabs', 'Side tabs', 'Tabs in a sidebar, with pinned sites as icons that remember their page.', 'mizu://settings/#tabs'],
  ['external', 'Links to apps', 'Steam, Telegram, Zoom and mail links open their app, after asking you.', 'mizu://settings/#privacy'],
  ['code', 'Mizu Scripts', 'Userscripts per site, with automatic updates.', 'mizu://scripts/'],
  ['puzzle', 'Extensions', 'Chromium extensions from the Chrome Web Store.', 'mizu://settings/#advanced'],
  ['media', 'Picture in picture', 'Pop any video into a floating window and switch videos there.', 'mizu://settings/#media'],
  ['private', 'Private windows', 'A separate in-memory session that leaves nothing behind.', null],
];

// Highlights of the current release line (the full list is in the changelog).
const WHATS_NEW = [
  ['import', 'Move in from another browser', 'Bookmarks, history and search engines from Chrome, Edge, Brave, Vivaldi, Opera or Firefox.'],
  ['folder', 'Tab groups', 'Named, coloured groups in both layouts; collapse them to a chip with a count.'],
  ['sparkle', 'Quick actions', 'Type > in the address bar: clear a site\u2019s data, mute all tabs, unload the rest and more.'],
  ['sliders', 'Units in the address bar', '10 km in mi, 70 f to c, 2 kg to lb: the answer appears as you type.'],
  ['drop', 'Welcome to Mizu', 'A short tour on first start: your style, default browser and import.'],
  ['music', 'Music and video hub', 'Every playing tab in one panel, with a volume per site and media keys.'],
];

function updateText(u, installed) {
  switch (u && u.state) {
    case 'checking':
      return 'Checking for updates…';
    case 'up_to_date':
      return 'Mizu is up to date.';
    case 'ready':
      return `Mizu ${u.version} is downloaded and verified. Restart to install it.`;
    case 'available':
      return `Mizu ${u.version} is available. This copy is not installed with MizuSetup, so download it from the releases page.`;
    case 'failed':
      return `Could not check for updates: ${u.error}`;
    default:
      return installed ? 'Updates are signed by Onibi Lab and installed automatically.' : 'Portable copy: updates are announced here; install Mizu with MizuSetup for automatic updates.';
  }
}

function updateRow(a) {
  const text = h('div', { class: 'd', text: updateText(a.update, a.installed) });
  const btn = h('button', { class: 'btn' });
  const set = (u) => {
    text.textContent = i18n.tr(updateText(u, a.installed));
    btn.disabled = u && u.state === 'checking';
    if (u && u.state === 'ready') {
      btn.className = 'btn primary';
      btn.textContent = i18n.tr('Restart to update');
      btn.onclick = () => mizu.request('update_install').catch((e) => showToast(e.message));
    } else if (u && u.state === 'available') {
      btn.className = 'btn primary';
      btn.textContent = i18n.tr('Releases');
      btn.onclick = () => openUrl(`${REPO}/releases/latest`);
    } else {
      btn.className = 'btn';
      btn.textContent = i18n.tr('Check now');
      btn.onclick = () => {
        set({ state: 'checking' });
        mizu.request('update_check').then(set).catch((e) => set({ state: 'failed', error: e.message }));
      };
    }
  };
  set(a.update);
  return h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'Mizu' }), text), btn);
}


// Crash reports stay on this PC; the user decides whether to attach one to an issue.
function crashRow() {
  const d = h('div', { class: 'd', text: 'Checking…' });
  const btn = h('button', { class: 'btn', text: 'Open folder' });
  btn.addEventListener('click', () => mizu.send('open_crash_folder'));
  mizu.request('crash_reports').then((r) => {
    const n = (r && r.count) || 0;
    d.textContent = i18n.tr(n ? `${n} saved on this PC. Nothing is sent anywhere; attach one to a GitHub issue if you like.` : 'No crashes recorded. If Mizu ever crashes, a report is saved here and never sent.');
    btn.hidden = !n;
  });
  return h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'Crash reports' }), d), btn);
}

function openUrl(url) {
  mizu.send('open_url', { url, disposition: 'new_tab' });
}

bootPage(() => {
  mizu.request('about').then((a) => {
    const rows = [
      ['Mizu', a.version],
      ['Engine', `${a.engine} ${a.engine_version || '(not found)'}`],
      ['Operating system', a.os],
      ['Profile folder', a.profile],
    ];
    const copy = h('button', { class: 'btn ghost' }, icon('copy'), h('span', { text: 'Copy' }));
    copy.addEventListener('click', () => {
      navigator.clipboard
        .writeText(rows.map(([k, v]) => `${k}: ${v}`).join('\n'))
        .then(() => showToast('Copied'));
    });

    const link = (iconName, label, url) => {
      const b = h('button', { class: 'about-link' }, icon(iconName), h('span', { text: label }));
      b.addEventListener('click', () => openUrl(url));
      return b;
    };

    main.replaceChildren(
      h('div', { class: 'page about' },
        h('section', { class: 'about-hero currents-hero', title: 'Let it happen' },
          currentsCanvas(),
          h('div', { class: 'hero-text' },
          h('div', { class: 'about-name' }, h('img', { src: '/_/logo.svg', alt: '' }), h('h1', { text: 'Mizu' }), h('span', { class: 'kanji', text: '水' })),
          h('div', { class: 'tag', text: `${a.tagline}.` }),
          h('div', { class: 'hero-motto', text: 'Go with the current.' }),
          h('div', { class: 'about-chips' },
            h('span', { class: 'pill accent', text: `Version ${a.version}` }),
            h('span', { class: 'pill', translate: 'no', text: `Chromium ${a.engine_version || '?'}` }),
            h('span', { class: 'pill', text: a.vendor })))),

        h('div', { class: 'about-grid' },
          ...FEATURES.map(([ic, title, text, url]) => {
            const tile = h(url ? 'button' : 'div', { class: `about-feature${url ? ' link' : ''}` }, h('div', { class: 'af-icon' }, icon(ic, { size: 20 })), h('div', { class: 'af-title', text: title }), h('div', { class: 'af-text', text }));
            if (url) tile.addEventListener('click', () => mizu.send('open_url', { url, disposition: 'current' }));
            return tile;
          }),
        ),

        h('section', { class: 'card whats-new', id: 'new' },
          h('h2', { text: "What's new" }),
          h('ul', { class: 'wn-list' },
            ...WHATS_NEW.map(([ic, title, text]) => h('li', null, h('span', { class: 'wn-icon' }, icon(ic, { size: 16 })), h('div', null, h('div', { class: 'wn-title', text: title }), h('div', { class: 'wn-text', text }))))),
        ),

        h('section', { class: 'card' },
          h('div', { class: 'card-head' }, h('h2', { text: 'System' }), copy),
          ...rows.map(([k, v]) => h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: k })), h('span', { class: k === 'Profile folder' ? 'mono' : '', translate: 'no', text: v, style: { userSelect: 'text' } })))),

        h('section', { class: 'card' },
          h('h2', { text: 'Updates' }),
          h('div', { class: 'setting' },
            h('div', { class: 'txt' },
              h('div', { class: 't', text: 'Web engine' }),
              h('div', { class: 'd', text: 'Microsoft Edge WebView2 is updated automatically by Windows, including security fixes.' })),
            h('span', { class: 'pill ok', text: 'Automatic' })),
          updateRow(a)),

        h('section', { class: 'card' },
          h('h2', { text: 'Privacy' }),
          h('p', { class: 'note', text: 'Mizu sends no analytics or telemetry to Onibi Lab. Your history, bookmarks, passwords and settings are stored only on this device. Filter lists are downloaded directly from their maintainers.' }),
          crashRow()),

        h('div', { class: 'about-links' },
          link('code', 'Source code', REPO),
          link('warning', 'Report a problem', `${REPO}/issues`),
          link('sparkle', 'What’s new', 'mizu://whatsnew/'),
          link('file', 'Changelog', `${REPO}/blob/main/CHANGELOG.md`),
          link('sparkle', 'Roadmap', `${REPO}/blob/main/ROADMAP.md`),
          link('download', 'All releases', `${REPO}/releases`)),

        h('p', { class: 'about-credits', text: 'Content blocking: adblock-rust and adblock-resources by Brave Software (MPL-2.0). Filter lists: uBlock Origin, EasyList, Peter Lowe and their contributors. Web engine: Microsoft Edge WebView2.' }),
        h('p', { class: 'about-legal', text: `© ${new Date().getFullYear()} Onibi Lab · Mozilla Public License 2.0` }),
      ),
    );
    if (location.hash === '#new') requestAnimationFrame(() => document.getElementById('new')?.scrollIntoView({ block: 'start' }));
  });
});
