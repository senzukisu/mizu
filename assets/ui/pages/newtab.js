// New tab page: search (with engine switch), bookmark tiles with "+", Shield.
'use strict';

const main = document.getElementById('main');
let data = {};
let info = {};
let engineId = null;

function greeting() {
  const hr = new Date().getHours();
  if (hr < 5) return 'Good night';
  if (hr < 12) return 'Good morning';
  if (hr < 18) return 'Good afternoon';
  return 'Good evening';
}

function searchUrl(text) {
  const e = (data.engines || []).find((x) => x.id === engineId);
  return e ? e.template.replace('{searchTerms}', encodeURIComponent(text).replace(/%20/g, '+')) : null;
}

function searchForm() {
  const input = h('input', { type: 'text', autofocus: true, 'aria-label': 'Search or address' });
  const engines = data.engines || [];
  const current = () => engines.find((e) => e.id === engineId) || { name: data.engine || '' };
  const setPlaceholder = () => (input.placeholder = current().name ? `Search ${current().name} or enter address` : 'Search or address');
  setPlaceholder();

  // Engine switch: a pill showing the engine's initial; click to pick another for this search.
  const pill = h('button', { type: 'button', class: 'nt-engine', title: 'Search engine' });
  const pillText = () => pill.replaceChildren(h('span', { text: (current().name || '?').charAt(0) }), icon('chevron-down', { size: 12 }));
  pillText();
  const menu = h('div', { class: 'nt-engines', hidden: true });
  for (const e of engines) {
    const b = h('button', { type: 'button', class: e.id === engineId ? 'on' : '' }, h('span', { class: 'ei', text: e.name.charAt(0) }), h('span', { text: e.name, translate: 'no' }));
    b.addEventListener('click', () => {
      engineId = e.id;
      menu.hidden = true;
      pillText();
      setPlaceholder();
      menu.querySelectorAll('button').forEach((x) => x.classList.toggle('on', x === b));
      input.focus();
    });
    menu.append(b);
  }
  pill.addEventListener('click', (ev) => {
    ev.stopPropagation();
    menu.hidden = !menu.hidden;
  });
  document.addEventListener('click', () => (menu.hidden = true));

  const form = h('form', { class: 'nt-search' }, engines.length ? pill : null, input, menu);
  form.addEventListener('submit', (e) => {
    e.preventDefault();
    const text = input.value.trim();
    if (!text) return;
    // A different engine than the default only applies to plain searches.
    const looksLikeUrl = /^[a-z][\w+.-]*:\/\//i.test(text) || (/^[^\s]+\.[a-z]{2,}(\/|$)/i.test(text) && !text.includes(' '));
    if (engineId && engineId !== data.engine_id && !looksLikeUrl) mizu.send('open_url', { url: searchUrl(text), disposition: 'current' });
    else mizu.send('navigate', { input: text });
  });
  setTimeout(() => input.focus(), 0);
  return form;
}

function tileDialog(tile) {
  const title = h('input', { type: 'text', value: tile ? tile.title : '', placeholder: 'Name' });
  const url = h('input', { type: 'url', value: tile ? tile.url : '', placeholder: 'https://example.com' });
  const dlg = h('dialog', null, h('h3', { text: tile ? 'Edit shortcut' : 'Add shortcut' }),
    h('div', { class: 'fields' }, h('label', null, 'Name', title), h('label', null, 'URL', url)),
    h('div', { class: 'actions' }, h('button', { class: 'btn', text: 'Cancel', onclick: () => dlg.close() }), h('button', { class: 'btn primary', text: 'Save', onclick: async () => {
      let u = url.value.trim();
      if (!u) return showToast('Enter an address');
      if (!/^[a-z][\w+.-]*:/i.test(u)) u = `https://${u}`;
      const name = title.value.trim() || hostOf(u) || u;
      try {
        if (tile) await mizu.request('bookmark_update', { id: tile.id, title: name, url: u });
        else await mizu.request('bookmark_add', { parent: 1, title: name, url: u });
        dlg.close();
        load();
      } catch (e) {
        showToast(e.message);
      }
    } })));
  document.body.append(dlg);
  dlg.showModal();
  dlg.addEventListener('close', () => dlg.remove());
  setTimeout(() => (tile ? title : url).focus(), 30);
}

function tiles() {
  const box = h('div', { class: 'nt-tiles' });
  for (const s of data.top || []) {
    const label = s.title || (s.host || hostOf(s.url) || s.url).replace(/^www\./, '');
    const a = h('a', { class: 'nt-tile', href: s.url, title: s.url, translate: 'no' }, letterAvatar(s.url, s.title), h('span', { text: label }));
    a.addEventListener('click', (e) => {
      e.preventDefault();
      mizu.send('open_url', { url: s.url, disposition: e.ctrlKey ? 'background' : 'current' });
    });
    a.addEventListener('auxclick', (e) => {
      if (e.button === 1) {
        e.preventDefault();
        mizu.send('open_url', { url: s.url, disposition: 'background' });
      }
    });
    const edit = h('button', { class: 'nt-tile-act', title: 'Edit' }, icon('edit', { size: 13 }));
    edit.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      tileDialog(s);
    });
    const del = h('button', { class: 'nt-tile-act', title: 'Remove' }, icon('close', { size: 13 }));
    del.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      mizu.request('bookmark_delete', { id: s.id }).then(load);
    });
    a.append(h('div', { class: 'nt-tile-acts' }, edit, del));
    box.append(a);
  }
  if (!info.private) {
    const add = h('button', { class: 'nt-tile add', title: 'Add shortcut' }, h('div', { class: 'fav' }, icon('plus', { size: 20 })), h('span', { text: 'Add' }));
    add.addEventListener('click', () => tileDialog(null));
    box.append(add);
  }
  return box;
}


function render() {
  const isPrivate = !!info.private || !!data.private || document.documentElement.hasAttribute('data-private');
  const clock = h('div', { class: 'nt-clock' });
  const date = h('div', { class: 'nt-greet' });
  const tick = () => {
    const now = new Date();
    clock.textContent = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const day = now.toLocaleDateString(i18n.lang === 'ru' ? 'ru-RU' : 'en-US', { weekday: 'long', day: 'numeric', month: 'long' });
    date.textContent = `${i18n.tr(greeting())} · ${day}`;
  };
  tick();
  clearInterval(render.timer);
  render.timer = setInterval(tick, 10000);

  const content = h('div', { class: 'nt' });
  if (isPrivate) {
    content.append(
      h('div', { class: 'nt-private' }, icon('private', { size: 44 }), h('h1', { text: 'Private window' }),
        h('p', { text: "Mizu won't save your history, cookies, site data or form entries from this window. They are deleted when you close the last private window." }),
        h('p', { class: 'small', text: 'Private browsing does not hide you from the sites you visit, your employer, school or internet provider. Downloads and bookmarks you create are kept.' })),
      searchForm(),
    );
  } else {
    content.append(h('div', { class: 'nt-hello' }, clock, date), searchForm());
  }

  if (data.crashed && data.restorable > 0) {
    const btn = h('button', { class: 'btn primary', text: `Restore ${data.restorable} tab${data.restorable === 1 ? '' : 's'}` });
    const banner = h('div', { class: 'nt-banner' }, icon('warning', { size: 18 }), h('span', { text: "Mizu didn't shut down correctly." }), btn);
    btn.addEventListener('click', () => {
      mizu.send('restore_session');
      banner.remove();
    });
    content.append(banner);
  }

  if (!isPrivate && data.engines) {
    content.append(tiles());
    const stats = h('a', { class: 'nt-stats', href: '#' }, icon('shield-check', { size: 16 }), h('span', { text: `${Number(data.blocked_lifetime || 0)} ads and trackers blocked by Mizu Shield` }));
    stats.addEventListener('click', (e) => {
      e.preventDefault();
      mizu.send('open_url', { url: 'mizu://shield/', disposition: 'current' });
    });
    content.append(stats);
  }
  main.replaceChildren(content, h('div', { class: 'nt-wave', 'aria-hidden': 'true' }));
}

function load() {
  return mizu
    .request('newtab_data')
    .then((d) => {
      data = d || {};
      if (!engineId) engineId = data.engine_id;
      render();
    })
    .catch(() => render());
}

// Draw immediately; fill in tiles when the browser answers.
render();
bootPage((i) => {
  info = i || {};
  load();
});
