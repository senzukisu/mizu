// About Mizu.
'use strict';

const main = document.getElementById('main');
const REPO = 'https://github.com/senzukisu/mizu';

const FEATURES = [
  ['shield-check', 'Mizu Shield', 'uBlock Origin–compatible ad and tracker blocking built into the browser.'],
  ['key', 'Passwords', 'Logins encrypted with your Windows account, filled from the address bar.'],
  ['code', 'Mizu Scripts', 'Tampermonkey-style userscripts with automatic updates.'],
  ['puzzle', 'Extensions', 'Chromium extensions straight from the Chrome Web Store.'],
  ['media', 'Picture in picture', 'Pop any video out into a floating window.'],
  ['private', 'Private windows', 'A separate in-memory session that leaves nothing behind.'],
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
        h('section', { class: 'about-hero' },
          h('img', { src: '/_/logo.svg', alt: '' }),
          h('div', { class: 'about-name' }, h('h1', { text: 'Mizu' }), h('span', { class: 'kanji', text: '水' })),
          h('div', { class: 'tag', text: `${a.tagline}.` }),
          h('div', { class: 'about-chips' },
            h('span', { class: 'pill accent', text: `Version ${a.version}` }),
            h('span', { class: 'pill', translate: 'no', text: `Chromium ${a.engine_version || '?'}` }),
            h('span', { class: 'pill', text: a.vendor }))),

        h('div', { class: 'about-grid' },
          ...FEATURES.map(([ic, title, text]) => h('div', { class: 'about-feature' }, h('div', { class: 'af-icon' }, icon(ic, { size: 20 })), h('div', { class: 'af-title', text: title }), h('div', { class: 'af-text', text }))),
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
          h('p', { class: 'note', text: 'Mizu sends no analytics or telemetry to Onibi Lab. Your history, bookmarks, passwords and settings are stored only on this device. Filter lists are downloaded directly from their maintainers.' })),

        h('div', { class: 'about-links' },
          link('code', 'Source code', REPO),
          link('warning', 'Report a problem', `${REPO}/issues`),
          link('file', 'Changelog', `${REPO}/blob/main/CHANGELOG.md`),
          link('sparkle', 'Roadmap', `${REPO}/blob/main/ROADMAP.md`)),

        h('p', { class: 'about-credits', text: 'Content blocking: adblock-rust and adblock-resources by Brave Software (MPL-2.0). Filter lists: uBlock Origin, EasyList, Peter Lowe and their contributors. Web engine: Microsoft Edge WebView2.' }),
        h('p', { class: 'about-legal', text: `© ${new Date().getFullYear()} Onibi Lab · Mozilla Public License 2.0` }),
      ),
    );
  });
});
