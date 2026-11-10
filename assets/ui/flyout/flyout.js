// Flyout surface: renders whichever menu/panel the browser asks for and reports its size.
'use strict';

const root = document.getElementById('root');
let kind = null;
let data = null;

const close = () => mizu.send('close_flyout');
const open = (url, disposition = 'new_tab') => {
  mizu.send('open_url', { url, disposition });
  close();
};

function reportSize() {
  requestAnimationFrame(() => {
    const r = root.getBoundingClientRect();
    mizu.send('flyout_size', { width: Math.ceil(r.width), height: Math.ceil(r.height) });
  });
}

function item(iconName, label, onClick, opts = {}) {
  const el = h('button', { class: `item${opts.danger ? ' danger' : ''}` }, iconName ? icon(iconName) : h('span', { style: { width: '16px' } }), h('span', { class: 'label', text: label }), opts.kbd ? h('span', { class: 'kbd', text: opts.kbd }) : null);
  el.addEventListener('click', () => {
    onClick();
    if (!opts.keepOpen) close();
  });
  return el;
}
const sep = () => h('div', { class: 'sep' });

// ── Main menu ────────────────────────────────────────────────
function renderMenu(d) {
  const zoomVal = h('span', { class: 'val', text: `${d.zoom || 100}%` });
  const zoomBtn = (name, delta, title) => {
    const b = h('button', { class: 'mini', title }, icon(name));
    b.addEventListener('click', () => {
      mizu.send('zoom', { delta });
      const steps = [25, 33, 50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200];
      let z = parseInt(zoomVal.textContent, 10) || 100;
      if (delta === 0) z = 100;
      else if (delta > 0) z = steps.find((s) => s > z) || 300;
      else z = [...steps].reverse().find((s) => s < z) || 25;
      zoomVal.textContent = `${z}%`;
    });
    return b;
  };
  const fs = h('button', { class: 'mini', title: 'Full screen (F11)' }, icon('fullscreen'));
  fs.addEventListener('click', () => {
    mizu.send('window', { action: 'fullscreen' });
    close();
  });
  return h(
    'div',
    { class: 'menu' },
    item('plus', 'New tab', () => mizu.send('new_tab'), { kbd: 'Ctrl+T' }),
    item('window', 'New window', () => mizu.send('new_window', { private: false }), { kbd: 'Ctrl+N' }),
    item('private', 'New private window', () => mizu.send('new_window', { private: true }), { kbd: 'Ctrl+Shift+N' }),
    sep(),
    h('div', { class: 'zoom-row' }, h('span', { class: 'label', text: 'Zoom' }), zoomBtn('zoom-out', -1, 'Zoom out (Ctrl+−)'), zoomVal, zoomBtn('zoom-in', 1, 'Zoom in (Ctrl++)'), fs),
    sep(),
    item('history', 'History', () => open('mizu://history/'), { kbd: 'Ctrl+H' }),
    item('download', 'Downloads', () => open('mizu://downloads/'), { kbd: 'Ctrl+J' }),
    item('bookmark', 'Bookmarks', () => open('mizu://bookmarks/'), { kbd: 'Ctrl+Shift+O' }),
    item('puzzle', 'Scripts & extensions', () => open('mizu://scripts/')),
    item('shield', 'Mizu Shield', () => open('mizu://shield/')),
    sep(),
    item('print', 'Print…', () => mizu.send('print'), { kbd: 'Ctrl+P' }),
    d.devtools ? item('devtools', 'Developer tools', () => mizu.send('devtools'), { kbd: 'F12' }) : null,
    item('gauge', 'Task manager', () => mizu.send('task_manager'), { kbd: 'Shift+Esc' }),
    sep(),
    item('settings', 'Settings', () => open('mizu://settings/')),
    item('info', 'About Mizu', () => open('mizu://about/')),
  );
}

// ── Tab context menu ─────────────────────────────────────────
function renderTabMenu(d) {
  const id = d.id;
  return h(
    'div',
    { class: 'menu', style: { width: '250px' } },
    item('plus', 'New tab', () => mizu.send('new_tab'), { kbd: 'Ctrl+T' }),
    sep(),
    item('reload', 'Reload', () => mizu.send('reload_tab', { id }), { kbd: 'Ctrl+R' }),
    item('copy', 'Duplicate', () => mizu.send('duplicate_tab', { id })),
    item('pin', d.pinned ? 'Unpin' : 'Pin', () => mizu.send('pin_tab', { id, pinned: !d.pinned })),
    item(d.muted ? 'volume' : 'mute', d.muted ? 'Unmute site' : 'Mute tab', () => mizu.send('mute_tab', { id, muted: !d.muted })),
    sep(),
    item('close', 'Close tab', () => mizu.send('close_tab', { id }), { kbd: 'Ctrl+W' }),
    d.count > 1 ? item(null, 'Close other tabs', () => mizu.send('close_other_tabs', { id })) : null,
    d.count > 1 ? item(null, 'Close tabs to the right', () => mizu.send('close_tabs_right', { id })) : null,
    d.closed ? item('history', 'Reopen closed tab', () => mizu.send('reopen_closed_tab'), { kbd: 'Ctrl+Shift+T' }) : null,
  );
}

// ── Downloads ────────────────────────────────────────────────
function downloadStatus(d) {
  switch (d.state) {
    case 'in_progress': {
      const parts = [d.total > 0 ? `${formatBytes(d.received)} of ${formatBytes(d.total)}` : formatBytes(d.received)];
      if (d.speed > 0) parts.push(`${formatBytes(d.speed)}/s`);
      if (d.eta > 0) parts.push(`${formatDuration(d.eta)} left`);
      return parts.join(' · ');
    }
    case 'paused':
      return `Paused · ${formatBytes(d.received)}${d.total > 0 ? ` of ${formatBytes(d.total)}` : ''}`;
    case 'completed':
      return d.exists ? `${formatBytes(d.total > 0 ? d.total : d.received)} · ${hostOf(d.url)}` : 'File moved or deleted';
    case 'cancelled':
      return 'Cancelled';
    case 'interrupted':
      return 'Interrupted when Mizu closed';
    default:
      return 'Failed';
  }
}

function downloadRow(d, compact) {
  const act = (name, title, action) => {
    const b = h('button', { class: 'mini', title }, icon(name));
    b.addEventListener('click', (e) => {
      e.stopPropagation();
      mizu.send('download_action', { id: d.id, action });
    });
    return b;
  };
  const acts = h('div', { class: 'acts' });
  if (d.state === 'in_progress' && d.live) acts.append(act('pause', 'Pause', 'pause'), act('close', 'Cancel', 'cancel'));
  else if (d.state === 'paused' && d.live) acts.append(act('play', 'Resume', 'resume'), act('close', 'Cancel', 'cancel'));
  else if (d.state === 'completed' && d.exists) acts.append(act('folder', 'Show in folder', 'show'));
  else if (d.state !== 'in_progress') acts.append(act('reload', 'Retry', 'retry'));
  const failed = ['failed', 'cancelled', 'interrupted'].includes(d.state);
  const name = h('div', { class: `name${d.state === 'completed' && d.exists ? ' done' : ''}`, text: d.file || d.url, title: d.path });
  if (d.state === 'completed' && d.exists) name.addEventListener('click', () => mizu.send('download_action', { id: d.id, action: 'open' }));
  const meta = h('div', { class: 'meta' }, name);
  if (d.state === 'in_progress' || d.state === 'paused') {
    const pct = d.total > 0 ? (100 * d.received) / d.total : 0;
    meta.append(h('div', { class: `bar${d.total > 0 ? '' : ' indeterminate'}` }, h('div', { style: { width: `${pct}%` } })));
  }
  meta.append(h('div', { class: `status${failed ? ' err' : ''}`, text: downloadStatus(d) }));
  return h('div', { class: 'dl' }, h('div', { class: 'ficon' }, icon('file')), meta, compact ? acts : acts);
}

function renderDownloads(items) {
  const list = (items || []).slice(0, 7);
  const panel = h('div', { class: 'panel' }, h('h2', null, h('span', { class: 'grow', text: 'Downloads' })));
  if (!list.length) panel.append(h('div', { class: 'empty', text: 'Nothing downloaded yet.' }));
  for (const d of list) panel.append(downloadRow(d, true));
  const all = h('button', { class: 'link', text: 'All downloads' });
  all.addEventListener('click', () => open('mizu://downloads/'));
  panel.append(h('div', { class: 'footer' }, all));
  return panel;
}

// ── Shield ───────────────────────────────────────────────────
function renderShield(d) {
  const toggle = h('button', { class: `toggle${d.enabled ? ' on' : ''}`, title: d.enabled ? 'Turn off for this site' : 'Turn on for this site', disabled: !d.global });
  toggle.addEventListener('click', () => {
    mizu.send('shield_toggle_site', { enabled: !d.enabled });
    close();
  });
  const level = { off: 'Off', basic: 'Basic', balanced: 'Balanced', strict: 'Strict' }[d.tracking_prevention] || d.tracking_prevention;
  const panel = h(
    'div',
    { class: 'panel' },
    h('h2', null, icon(d.enabled ? 'shield-check' : 'shield-off', { size: 20 }), h('span', { class: 'grow', text: d.host || 'Mizu Shield' }), toggle),
    d.global
      ? h('div', { class: 'big' }, h('b', { text: String(d.blocked || 0) }), h('span', { class: 'muted', text: d.enabled ? 'ads and trackers blocked on this page' : 'Shield is off for this site' }))
      : h('div', { class: 'big' }, h('span', { class: 'muted', text: 'Content blocking is turned off in Settings.' })),
    h('div', { class: 'rows' }, h('div', { class: 'row' }, h('span', { class: 'k', text: 'Tracking prevention (engine)' }), h('span', { class: 'v', text: level })), ...(d.top || []).slice(0, 4).map(([host, n]) => h('div', { class: 'row' }, h('span', { class: 'k', text: host }), h('span', { class: 'v', text: String(n) })))),
  );
  const dash = h('button', { class: 'link', text: 'Shield dashboard' });
  dash.addEventListener('click', () => open('mizu://shield/'));
  panel.append(h('div', { class: 'footer' }, dash));
  return panel;
}

// ── Site info ────────────────────────────────────────────────
const PERM_ICONS = { camera: 'camera', microphone: 'mic', geolocation: 'location', notifications: 'bell', clipboard_read: 'clipboard' };
function renderSiteInfo(d) {
  const panel = h(
    'div',
    { class: 'panel' },
    h('h2', null, icon(d.secure ? 'lock' : 'warning', { size: 18 }), h('span', { class: 'grow', text: d.host || d.url })),
    h('div', { class: 'muted', style: { margin: '0 4px 10px' }, text: d.private ? 'Private window: nothing from this site is kept after you close it.' : d.secure ? 'Connection is secure. Information you send to this site is encrypted.' : 'Connection is not secure. Do not enter passwords or payment details here.' }),
  );
  if (d.permissions?.length) {
    const rows = h('div', { class: 'rows' });
    for (const p of d.permissions) {
      const reset = h('button', { class: 'link', text: 'Reset' });
      reset.addEventListener('click', () => {
        mizu.request('permission_set', { origin: d.origin, kind: p.kind, state: 'reset' }).then(() => reset.closest('.row').remove());
      });
      rows.append(h('div', { class: 'row' }, h('span', { class: 'k' }, icon(PERM_ICONS[p.kind] || 'key', { size: 14 }), ` ${p.kind.replace(/_/g, ' ')}: ${p.state}`), reset));
    }
    panel.append(rows);
  }
  const settings = h('button', { class: 'link', text: 'Site permissions' });
  settings.addEventListener('click', () => open('mizu://settings/#privacy'));
  panel.append(h('div', { class: 'footer' }, settings));
  return panel;
}

// ── Bookmark editor ──────────────────────────────────────────
function renderBookmark(d) {
  const bm = d.bookmark;
  if (!bm) return h('div', { class: 'panel' }, h('div', { class: 'empty', text: 'This page cannot be bookmarked.' }));
  const title = h('input', { value: bm.title });
  const folder = h('select');
  for (const f of d.folders || []) {
    const o = h('option', { value: f.id, text: f.title });
    if (f.id === bm.parent_id) o.selected = true;
    folder.append(o);
  }
  const remove = h('button', { class: 'btn', text: 'Remove' });
  remove.addEventListener('click', () => {
    mizu.request('bookmark_delete', { id: bm.id }).finally(close);
  });
  const done = h('button', { class: 'btn primary', text: 'Done' });
  const save = async () => {
    await mizu.request('bookmark_update', { id: bm.id, title: title.value.trim() || bm.title, url: null });
    const parent = Number(folder.value);
    if (parent && parent !== bm.parent_id) await mizu.request('bookmark_move', { id: bm.id, parent, index: 1e6 });
    close();
  };
  done.addEventListener('click', save);
  title.addEventListener('keydown', (e) => e.key === 'Enter' && save());
  setTimeout(() => {
    title.focus();
    title.select();
  }, 30);
  return h(
    'div',
    { class: 'panel', style: { width: '320px' } },
    h('h2', null, icon('star-filled', { size: 18 }), h('span', { class: 'grow', text: 'Bookmark added' })),
    h('div', { class: 'field' }, h('label', { text: 'Name' }), title),
    h('div', { class: 'field' }, h('label', { text: 'Folder' }), folder),
    h('div', { class: 'footer' }, remove, done),
  );
}

// ── Omnibox suggestions ──────────────────────────────────────
const SUG_ICONS = { navigate: 'globe', search: 'search', history: 'history', bookmark: 'star', internal: 'drop' };
function renderOmnibox(d) {
  const box = h('div', { class: 'omni', style: { width: `${Math.max(320, Math.round(d.width || 560)) - 12}px` } });
  (d.items || []).forEach((s, i) => {
    const row = h('div', { class: `sug${i === d.selected ? ' sel' : ''}` }, icon(SUG_ICONS[s.kind] || 'globe'), h('span', { class: 't', text: s.title }), s.display && s.display !== s.title ? h('span', { class: 'd', text: s.display }) : null);
    row.addEventListener('mousedown', (e) => e.preventDefault());
    row.addEventListener('click', () => mizu.send('omnibox_commit', { index: i }));
    box.append(row);
  });
  return box;
}

// ── Permission prompt ────────────────────────────────────────
function renderPermission(d) {
  const remember = h('input', { type: 'checkbox', checked: !d.private });
  const answer = (allow) => mizu.send('permission_response', { prompt_id: d.prompt_id, allow, remember: remember.checked });
  const block = h('button', { class: 'btn', text: 'Block' });
  block.addEventListener('click', () => answer(false));
  const allow = h('button', { class: 'btn primary', text: 'Allow' });
  allow.addEventListener('click', () => answer(true));
  return h(
    'div',
    { class: 'perm' },
    h('div', { class: 'q' }, h('div', { class: 'ic' }, icon(PERM_ICONS[d.kind] || 'key', { size: 18 })), h('div', null, h('b', { text: d.host || d.origin }), h('div', { text: ` wants to ${d.description}` }))),
    h('label', { class: 'check' }, remember, d.private ? 'Remember until private windows are closed' : 'Remember this decision'),
    h('div', { class: 'footer' }, block, allow),
  );
}

// ── Confirmation (closing many tabs) ─────────────────────────
function renderConfirm(d) {
  const skip = h('input', { type: 'checkbox' });
  const cancel = h('button', { class: 'btn', text: 'Cancel' });
  cancel.addEventListener('click', () => mizu.send('confirm_response', { accepted: false }));
  const ok = h('button', { class: 'btn danger', text: d.confirm || 'OK' });
  ok.addEventListener('click', () => mizu.send('confirm_response', { accepted: true, dont_ask_again: skip.checked }));
  setTimeout(() => ok.focus(), 30);
  return h(
    'div',
    { class: 'confirm' },
    h('div', { class: 'q' }, h('div', { class: 'ic' }, icon('tabs', { size: 20 })), h('div', null, h('div', { class: 'ct', text: d.title }), h('div', { class: 'muted', text: d.message }))),
    d.can_skip ? h('label', { class: 'check' }, skip, "Don't ask again") : null,
    h('div', { class: 'footer' }, cancel, ok),
  );
}

const RENDERERS = {
  confirm: renderConfirm,
  menu: renderMenu,
  tab_menu: renderTabMenu,
  downloads: renderDownloads,
  shield: renderShield,
  site_info: renderSiteInfo,
  bookmark: renderBookmark,
  omnibox: renderOmnibox,
  permission: renderPermission,
};

mizu.on('flyout', (m) => {
  if (m.data === null || m.data === undefined) {
    if (m.kind !== 'omnibox' && m.kind !== 'menu' && m.kind !== 'tab_menu') {
      // Panels without data still render (e.g. empty downloads list).
    } else {
      kind = null;
      root.replaceChildren();
      return;
    }
  }
  kind = m.kind;
  data = m.data;
  const render = RENDERERS[kind];
  if (!render) return;
  root.style.animation = 'none';
  void root.offsetWidth;
  root.style.animation = '';
  root.replaceChildren(render(data || {}));
  reportSize();
});

mizu.on('downloads', (m) => {
  if (kind === 'downloads') {
    root.replaceChildren(renderDownloads(m.items));
    reportSize();
  }
});

mizu.on('settings', (m) => theme.apply(m.settings));

document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') {
    e.preventDefault();
    if (kind === 'permission' && data) mizu.send('permission_response', { prompt_id: data.prompt_id, allow: false, remember: false });
    else close();
  }
});
document.addEventListener('contextmenu', (e) => {
  if (!(e.target instanceof HTMLInputElement)) e.preventDefault();
});

mizu.request('ready').then((info) => theme.apply(info.settings, { private: info.private }));
