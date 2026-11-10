// New tab page: search, most visited sites, Shield summary, crash recovery.
'use strict';

const main = document.getElementById('main');

function greeting() {
  const hr = new Date().getHours();
  if (hr < 5) return 'Good night';
  if (hr < 12) return 'Good morning';
  if (hr < 18) return 'Good afternoon';
  return 'Good evening';
}

function render(info, d) {
  const isPrivate = !!(info && info.private) || !!d.private;
  const clock = h('div', { class: 'nt-clock' });
  const tick = () => (clock.textContent = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }));
  tick();
  setInterval(tick, 10000);

  const input = h('input', { type: 'text', placeholder: `Search ${d.engine || 'the web'} or enter address`, autofocus: true, 'aria-label': 'Search or address' });
  const form = h('form', { class: 'nt-search' }, icon('search', { size: 20 }), input);
  form.addEventListener('submit', (e) => {
    e.preventDefault();
    if (input.value.trim()) mizu.send('navigate', { input: input.value });
  });

  const content = h('div', { class: 'nt' });
  if (isPrivate) {
    content.append(
      h('div', { class: 'nt-private' }, icon('private', { size: 44 }), h('h1', { text: 'Private window' }),
        h('p', { text: 'Mizu won\'t save your history, cookies, site data or form entries from this window. They are deleted when you close the last private window.' }),
        h('p', { class: 'small', text: 'Private browsing does not hide you from the sites you visit, your employer, school or internet provider. Downloads and bookmarks you create are kept.' })),
      form,
    );
  } else {
    content.append(h('div', { class: 'nt-hello' }, clock, h('div', { class: 'nt-greet', text: greeting() })), form);
  }

  if (d.crashed && d.restorable > 0) {
    const btn = h('button', { class: 'btn primary', text: `Restore ${d.restorable} tab${d.restorable === 1 ? '' : 's'}` });
    btn.addEventListener('click', () => {
      mizu.send('restore_session');
      banner.remove();
    });
    const banner = h('div', { class: 'nt-banner' }, icon('warning', { size: 18 }), h('span', { text: 'Mizu didn\'t shut down correctly.' }), btn);
    content.append(banner);
  }

  if (!isPrivate && d.top && d.top.length) {
    const tiles = h('div', { class: 'nt-tiles' });
    for (const s of d.top) {
      const label = (s.host || hostOf(s.url) || s.title || s.url).replace(/^www\./, '');
      const a = h('a', { class: 'nt-tile', href: s.url, title: s.title ? `${s.title}
${s.url}` : s.url }, letterAvatar(s.url, s.title), h('span', { text: label }));
      a.addEventListener('click', (e) => {
        e.preventDefault();
        mizu.send('open_url', { url: s.url, disposition: e.ctrlKey ? 'background' : 'current' });
      });
      tiles.append(a);
    }
    content.append(tiles);
  }

  if (!isPrivate) {
    const stats = h('a', { class: 'nt-stats', href: '#' }, icon('shield-check', { size: 16 }), h('span', { text: `${Number(d.blocked_lifetime || 0).toLocaleString()} ads and trackers blocked by Mizu Shield` }));
    stats.addEventListener('click', (e) => {
      e.preventDefault();
      mizu.send('open_url', { url: 'mizu://shield/', disposition: 'current' });
    });
    content.append(stats);
  }
  main.replaceChildren(content, h('div', { class: 'nt-wave', 'aria-hidden': 'true' }));
  input.focus();
}

// Draw immediately; fill in sites and statistics when the browser answers.
render({ private: document.documentElement.hasAttribute('data-private') }, {});
bootPage((info) => {
  mizu
    .request('newtab_data')
    .then((d) => render(info, d))
    .catch(() => render(info, {}));
});
