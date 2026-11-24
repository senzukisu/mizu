// Import from another browser: pick a profile found on this PC, see what it holds, choose
// what to bring over. Passwords come from the browser's own CSV export (Mizu Passwords).
'use strict';

const main = document.getElementById('main');
let sources = [];
let chosen = null;
const what = { bookmarks: true, history: true, engines: true, to_bar: false };

const BROWSER_ICONS = { Firefox: 'globe' };

function card(title, ...rows) {
  return h('section', { class: 'card' }, title ? h('h2', { text: title }) : null, ...rows);
}

function render() {
  const list = h('div', { class: 'imp-sources' });
  if (!sources.length) {
    list.append(h('div', { class: 'empty-state' }, icon('import'), h('div', { text: 'No other browsers with data were found on this PC.' })));
  }
  for (const s of sources) {
    const total = s.counts.bookmarks + s.counts.history + s.counts.engines;
    const b = h('button', { class: `imp-src${chosen && chosen.id === s.id ? ' on' : ''}`, disabled: total === 0 },
      h('div', { class: 'imp-ic' }, icon(BROWSER_ICONS[s.browser] || 'globe', { size: 20 })),
      h('div', { class: 'imp-meta' },
        h('div', { class: 't', translate: 'no', text: s.browser }),
        h('div', { class: 'd', translate: 'no', text: s.profile })),
      h('div', { class: 'imp-counts' },
        h('span', { text: `${s.counts.bookmarks} bookmarks` }),
        h('span', { text: `${s.counts.history} history` }),
        s.kind === 'chromium' ? h('span', { text: `${s.counts.engines} search engines` }) : null));
    b.addEventListener('click', () => {
      chosen = s;
      render();
    });
    list.append(b);
  }

  const parts = [];
  parts.push(card('Import from', h('p', { class: 'note', text: 'Mizu reads a copy of the browser’s files; the browser itself is not changed and can stay open.' }), list));

  if (chosen) {
    const check = (key, label, n, note) => {
      const input = h('input', { type: 'checkbox' });
      input.checked = !!what[key] && n > 0;
      input.disabled = n === 0;
      input.addEventListener('change', () => (what[key] = input.checked));
      return h('label', { class: 'setting imp-check' }, h('div', { class: 'txt' }, h('div', { class: 't', text: label }), h('div', { class: 'd', text: note })), input);
    };
    const go = h('button', { class: 'btn primary', text: 'Import' });
    const status = h('span', { class: 'imp-status' });
    go.addEventListener('click', async () => {
      go.disabled = true;
      status.textContent = 'Importing…';
      try {
        const r = await mizu.request('import_run', { id: chosen.id, bookmarks: what.bookmarks, history: what.history, engines: what.engines, to_bar: what.to_bar && what.bookmarks, bar_name: i18n.tr('Bookmarks bar') });
        status.textContent = '';
        done(r);
      } catch (e) {
        status.textContent = e.message;
        go.disabled = false;
      }
    });
    parts.push(card(`From ${chosen.browser} · ${chosen.profile}`,
      check('bookmarks', 'Bookmarks', chosen.counts.bookmarks, 'Into an "Imported from…" folder in Other bookmarks; its bookmarks bar becomes a subfolder there.'),
      check('to_bar', 'Also put its bar on Mizu’s bookmarks bar', chosen.counts.bookmarks, 'Off by default: Mizu’s bar shows only what you put there.'),
      check('history', 'History', chosen.counts.history, 'Recent pages, merged with Mizu’s history. Address bar suggestions use them right away.'),
      chosen.kind === 'chromium' ? check('engines', 'Search engines', chosen.counts.engines, 'Added to your list in Settings → Search; your default engine stays the same.') : null,
      h('div', { class: 'setting' }, h('div', { class: 'txt' }, status), go)));
  }

  const pw = h('button', { class: 'btn' }, icon('key'), h('span', { text: 'Open Passwords' }));
  pw.addEventListener('click', () => mizu.send('open_url', { url: 'mizu://passwords/', disposition: 'current' }));
  parts.push(card('Passwords', h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'Bring your passwords too' }), h('div', { class: 'd', text: 'Export them to a CSV file in the other browser (its password settings have "Export"), then use Import in Mizu Passwords. Delete the CSV afterwards: it is not encrypted.' })), pw)));

  main.replaceChildren(h('div', { class: 'page', style: { maxWidth: '860px' } }, pageHead('Import from another browser'), ...parts));
}

function done(r) {
  const lines = [];
  if (r.bookmarks) lines.push(`${r.bookmarks} bookmarks`);
  if (r.history) lines.push(`${r.history} history entries`);
  if (r.engines) lines.push(`${r.engines} search engines`);
  const open = h('button', { class: 'btn primary', text: 'Open bookmarks' });
  open.addEventListener('click', () => mizu.send('open_url', { url: 'mizu://bookmarks/', disposition: 'current' }));
  const again = h('button', { class: 'btn', text: 'Import something else' });
  again.addEventListener('click', () => {
    chosen = null;
    load();
  });
  main.replaceChildren(h('div', { class: 'page', style: { maxWidth: '860px' } }, pageHead('Import from another browser'),
    card(null, h('div', { class: 'imp-done' }, icon('check', { size: 28 }), h('h3', { text: 'Done' }), h('p', { text: lines.length ? `Imported: ${lines.join(', ')}.` : 'Nothing new to import.' }), h('div', { class: 'row' }, again, open)))));
}

function load() {
  main.replaceChildren(h('div', { class: 'page' }, pageHead('Import from another browser'), h('div', { class: 'empty-state' }, h('div', { text: 'Looking for browsers…' }))));
  mizu.request('import_sources').then((list) => {
    sources = list || [];
    if (!chosen && sources.length) chosen = sources.find((s) => s.counts.bookmarks + s.counts.history > 0) || null;
    render();
  });
}

bootPage(load);
