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

// "Today", "Yesterday" or the date, for grouping.
function dayLabel(ms) {
  if (!ms) return 'Earlier';
  const d = new Date(ms);
  const start = (x) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const days = Math.round((start(new Date()) - start(d)) / 86400000);
  if (days === 0) return 'Today';
  if (days === 1) return 'Yesterday';
  return d.toLocaleDateString(document.documentElement.lang || undefined, { day: 'numeric', month: 'long', year: d.getFullYear() === new Date().getFullYear() ? undefined : 'numeric' });
}

function renderSummary() {
  const active = items.filter((d) => d.state === 'in_progress');
  summary.hidden = !active.length;
  if (!active.length) return;
  const speed = active.reduce((a, d) => a + (d.speed || 0), 0);
  const left = Math.max(0, ...active.map((d) => d.eta || 0));
  const parts = [active.length === 1 ? '1 downloading' : `${active.length} downloading`];
  if (speed > 0) parts.push(`${formatBytes(speed)}/s`);
  if (left > 0) parts.push(`${formatDuration(left)} left`);
  summary.replaceChildren(icon('download'), h('span', { text: parts.join(' · ') }));
}

function render() {
  renderSummary();
  list.replaceChildren();
  const q = query.toLowerCase();
  const shown = items.filter((d) => !q || (d.file || '').toLowerCase().includes(q) || (d.url || '').toLowerCase().includes(q));
  if (!shown.length) {
    list.append(h('div', { class: 'empty-state' }, icon('download'), h('div', { text: q ? 'No downloads match.' : 'Files you download will appear here.' })));
    return;
  }
  let lastDay = null;
  for (const d of shown) {
    const day = dayLabel(d.started);
    if (day !== lastDay) {
      lastDay = day;
      list.append(h('div', { class: 'dl-day', text: day }));
    }
    const act = (name, title, action) => {
      const b = h('button', { class: 'btn ghost', title }, icon(name), h('span', { text: title }));
      b.addEventListener('click', () => mizu.send('download_action', { id: d.id, action }));
      return b;
    };
    const acts = h('div', { class: 'dl-acts' });
    if (d.live && d.state === 'in_progress') acts.append(act('pause', 'Pause', 'pause'), act('close', 'Cancel', 'cancel'));
    if (d.live && d.state === 'paused') acts.append(act('play', 'Resume', 'resume'), act('close', 'Cancel', 'cancel'));
    if (d.state === 'completed' && d.exists) acts.append(act('external', 'Open', 'open'), d.viewable ? act('window', 'Open in Windows app', 'open_system') : null, act('more', 'Open with…', 'open_with'), act('folder', 'Show in folder', 'show'));
    if (['failed', 'cancelled', 'interrupted'].includes(d.state) || (d.state === 'completed' && !d.exists)) acts.append(act('reload', 'Retry', 'retry'));
    if (!['in_progress', 'paused'].includes(d.state)) {
      const rm = h('button', { class: 'icon-btn', title: 'Remove from list' }, icon('close'));
      rm.addEventListener('click', () => mizu.send('download_action', { id: d.id, action: 'remove' }));
      acts.append(rm);
    }
    const meta = h('div', { class: 'main' }, h('div', { class: 't', text: d.file || d.url, translate: 'no' }));
    const busy = d.state === 'in_progress' || d.state === 'paused';
    if (busy) {
      const pct = d.total > 0 ? (100 * d.received) / d.total : 0;
      meta.append(h('div', { class: `pbar${d.total > 0 ? '' : ' ind'}` }, h('div', { style: { width: `${pct}%` } })));
    }
    // One quiet line: where it came from and how it went.
    const err = ['failed', 'interrupted'].includes(d.state);
    meta.append(h('div', { class: 'u' }, h('span', { title: d.url, translate: 'no', text: hostOf(d.url) || d.url }), h('span', { text: ' · ' }), h('span', { class: err ? 'err' : '', text: status(d) })));
    list.append(h('div', { class: 'list-row dl-row' }, fileBadge(d.file || d.url), meta, acts));
  }
}

const clear = h('button', { class: 'btn', text: 'Clear list' });
clear.addEventListener('click', () => mizu.send('downloads_clear'));
const openFolder = h('button', { class: 'btn' }, icon('folder'), h('span', { text: 'Open folder' }));
openFolder.addEventListener('click', () => mizu.send('downloads_open_folder'));
const summary = h('div', { class: 'dl-summary', hidden: true });
main.append(h('div', { class: 'page' }, pageHead('Downloads', searchBox('Search downloads', (q) => {
  query = q;
  render();
}), openFolder, clear), summary, list));

mizu.on('downloads', (m) => {
  items = m.items || [];
  render();
});
bootPage(() => mizu.request('downloads_get').then((d) => {
  items = d || [];
  render();
}));
