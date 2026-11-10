// About Mizu: versions of the browser and its engine.
'use strict';

const main = document.getElementById('main');

bootPage(() => {
  mizu.request('about').then((a) => {
    const rows = [
      ['Mizu', `${a.version}`],
      ['Engine', `${a.engine} ${a.engine_version || '(not found)'}`],
      ['Operating system', a.os],
      ['Profile folder', a.profile],
    ];
    const table = h('div', { class: 'rows-card' }, ...rows.map(([k, v]) => h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: k })), h('span', { class: k === 'Profile folder' ? 'mono' : '', text: v, style: { userSelect: 'text' } }))));
    main.replaceChildren(
      h('div', { class: 'page about' },
        h('div', { class: 'about-hero' }, h('img', { src: '/_/logo.svg', alt: '' }), h('h1', { text: 'Mizu' }), h('div', { class: 'tag', text: `${a.tagline}.` }), h('div', { class: 'muted', text: `Version ${a.version} · ${a.vendor}` })),
        h('section', { class: 'card' }, table),
        h('section', { class: 'card' },
          h('h2', { text: 'Updates' }),
          h('p', { class: 'note', text: 'The web engine (Microsoft Edge WebView2) is kept up to date automatically by Windows, including security fixes. Mizu 0.1.0 itself does not yet update automatically: new versions are published on the project page.' })),
        h('section', { class: 'card' },
          h('h2', { text: 'Privacy' }),
          h('p', { class: 'note', text: 'Mizu sends no analytics or telemetry to Onibi Lab. Your history, bookmarks and settings are stored only on this device. Filter lists are downloaded directly from their maintainers.' })),
        h('p', { class: 'muted', style: { textAlign: 'center' }, text: `© ${new Date().getFullYear()} Onibi Lab · Licensed under the Mozilla Public License 2.0 · Content blocking by adblock-rust (Brave, MPL-2.0)` }),
      ),
    );
  });
});
