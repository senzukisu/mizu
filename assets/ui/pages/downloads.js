// Downloads page: live progress, speed, time left, pause/resume/cancel/retry, open, show.
'use strict';

const main = document.getElementById('main');
const list = h('section', { class: 'card' });
let items = [];
let query = '';

function status(d) {
  switch (d.state) {
    case 'in_progress': {
      const parts = [d.total > 0 ? `${formatBytes(d.received)} of ${formatBytes(d.total)}` : formatBytes(d.received)];
      if (d.speed > 0) parts.push(`${formatBytes(d.speed)}/s`);
      if (d.eta > 0) parts.push(`${formatDuration(d.eta)} left`);
      return parts.join(' · ');
    }
    case 'paused':
      return 'Paused';
    case 'completed':
      return d.exists ? formatBytes(d.total > 0 ? d.total : d.received) : 'Deleted or moved';
    case 'cancelled':
      return 'Cancelled';
    case 'interrupted':
      return 'Interrupted — Mizu was closed during the download';
    default:
      return 'Failed';
  }
}

function render() {
  list.replaceChildren();
  const q = query.toLowerCase();
  const shown = items.filter((d) => !q || (d.file || '').toLowerCase().includes(q) || (d.url || '').toLowerCase().includes(q));
  if (!shown.length) {
    list.append(h('div', { class: 'empty-state' }, icon('download'), h('div', { text: q ? 'No downloads match.' : 'Files you download will appear here.' })));
    return;
  }
  for (const d of shown) {
    const act = (name, title, action) => {
      const b = h('button', { class: 'btn ghost', title }, icon(name), h('span', { text: title }));
      b.addEventListener('click', () => mizu.send('download_action', { id: d.id, action }));
      return b;
    };
    const acts = h('div', { class: 'dl-acts' });
    if (d.live && d.state === 'in_progress') acts.append(act('pause', 'Pause', 'pause'), act('close', 'Cancel', 'cancel'));
    if (d.live && d.state === 'paused') acts.append(act('play', 'Resume', 'resume'), act('close', 'Cancel', 'cancel'));
    if (d.state === 'completed' && d.exists) acts.append(act('external', 'Open', 'open'), act('folder', 'Show in folder', 'show'));
    if (['failed', 'cancelled', 'interrupted'].includes(d.state) || (d.state === 'completed' && !d.exists)) acts.append(act('reload', 'Retry', 'retry'));
    if (!['in_progress', 'paused'].includes(d.state)) {
      const rm = h('button', { class: 'icon-btn', title: 'Remove from list' }, icon('close'));
      rm.addEventListener('click', () => mizu.send('download_action', { id: d.id, action: 'remove' }));
      acts.append(rm);
    }
    const meta = h('div', { class: 'main' }, h('div', { class: 't', text: d.file || d.url }), h('div', { class: 'u', title: d.url, text: hostOf(d.url) || d.url }));
    if (d.state === 'in_progress' || d.state === 'paused') {
      const pct = d.total > 0 ? (100 * d.received) / d.total : 0;
      meta.append(h('div', { class: `pbar${d.total > 0 ? '' : ' ind'}` }, h('div', { style: { width: `${pct}%` } })));
    }
    meta.append(h('div', { class: `u${['failed', 'interrupted'].includes(d.state) ? ' err' : ''}`, text: status(d) }));
    list.append(h('div', { class: 'list-row dl-row' }, h('div', { class: 'fav' }, icon('file')), meta, acts));
  }
}

const clear = h('button', { class: 'btn', text: 'Clear list' });
clear.addEventListener('click', () => mizu.send('downloads_clear'));
main.append(h('div', { class: 'page' }, pageHead('Downloads', searchBox('Search downloads', (q) => {
  query = q;
  render();
}), clear), list));

mizu.on('downloads', (m) => {
  items = m.items || [];
  render();
});
bootPage(() => mizu.request('downloads_get').then((d) => {
  items = d || [];
  render();
}));
