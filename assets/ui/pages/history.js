// History: grouped by day, searchable, per-entry removal and clearing by period.
'use strict';

const main = document.getElementById('main');
const list = h('section', { class: 'card' });
let query = '';
let offset = 0;
let exhausted = false;
let loading = false;
let lastDay = '';

function dayLabel(ms) {
  const d = new Date(ms);
  const today = new Date();
  const yesterday = new Date(Date.now() - 864e5);
  if (d.toDateString() === today.toDateString()) return 'Today';
  if (d.toDateString() === yesterday.toDateString()) return 'Yesterday';
  return d.toLocaleDateString([], { weekday: 'long', day: 'numeric', month: 'long', year: d.getFullYear() === today.getFullYear() ? undefined : 'numeric' });
}

function row(e) {
  const del = h('button', { class: 'icon-btn', title: 'Remove from history' }, icon('trash'));
  const r = h('div', { class: 'list-row' },
    letterAvatar(e.url, e.title),
    h('a', { class: 'main', href: e.url, title: e.url }, h('div', { class: 't', text: e.title || e.url }), h('div', { class: 'u', text: e.url })),
    h('span', { class: 'when', text: new Date(e.last_visit).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) }),
    h('div', { class: 'acts' }, del));
  r.querySelector('a').addEventListener('click', (ev) => {
    ev.preventDefault();
    mizu.send('open_url', { url: e.url, disposition: ev.ctrlKey || ev.button === 1 ? 'background' : 'current' });
  });
  del.addEventListener('click', () => mizu.request('history_delete', { id: e.id }).then(() => r.remove()));
  return r;
}

async function load(reset) {
  if (loading) return;
  if (reset) {
    offset = 0;
    exhausted = false;
    lastDay = '';
    list.replaceChildren();
  }
  if (exhausted) return;
  loading = true;
  try {
    const entries = await mizu.request('history_query', { query, offset });
    offset += entries.length;
    if (entries.length < 100) exhausted = true;
    for (const e of entries) {
      const day = dayLabel(e.last_visit);
      if (day !== lastDay) {
        lastDay = day;
        list.append(h('div', { class: 'group-label', text: day }));
      }
      list.append(row(e));
    }
    if (offset === 0) {
      list.append(h('div', { class: 'empty-state' }, icon('history'), h('div', { text: query ? 'No pages match your search.' : 'Pages you visit will appear here. Private windows are never recorded.' })));
    }
  } finally {
    loading = false;
  }
}

function clearDialog() {
  let range = 'hour';
  const sel = h('select');
  for (const [v, l] of [['hour', 'Last hour'], ['day', 'Last 24 hours'], ['week', 'Last 7 days'], ['month', 'Last 4 weeks'], ['all', 'All time']]) sel.append(h('option', { value: v, text: l }));
  sel.addEventListener('change', () => (range = sel.value));
  const dlg = h('dialog', null, h('h3', { text: 'Clear history' }), h('div', { class: 'fields' }, h('label', null, 'Time range', sel)),
    h('p', { class: 'muted', text: 'Cookies, cache and other site data can be cleared in Settings → Privacy.' }),
    h('div', { class: 'actions' }, h('button', { class: 'btn', text: 'Cancel', onclick: () => dlg.close() }), h('button', { class: 'btn danger', text: 'Clear', onclick: () => {
      mizu.request('history_clear', { range }).then((r) => {
        showToast(`${r.removed} visit${r.removed === 1 ? '' : 's'} removed`);
        dlg.close();
        load(true);
      });
    } })));
  document.body.append(dlg);
  dlg.showModal();
  dlg.addEventListener('close', () => dlg.remove());
}

const clearBtn = h('button', { class: 'btn', text: 'Clear history…' });
clearBtn.addEventListener('click', clearDialog);
main.append(h('div', { class: 'page' }, pageHead('History', searchBox('Search history', (q) => {
  query = q;
  load(true);
}), clearBtn), list));

window.addEventListener('scroll', () => {
  if (window.innerHeight + window.scrollY > document.body.scrollHeight - 400) load(false);
});
bootPage(() => load(true));
