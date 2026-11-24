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

// Measure synchronously: the flyout window is hidden until it knows its size, and hidden
// web views do not run requestAnimationFrame callbacks.
function reportSize() {
  // The popup's current size caps an inline-block root; content wider than that (the
  // suggestions list follows the address bar width) shows up in scrollWidth.
  const r = root.getBoundingClientRect();
  const width = Math.max(r.width, root.scrollWidth);
  mizu.send('flyout_size', { width: Math.ceil(width), height: Math.ceil(r.height) });
}

const GROUP_COLOR_NAMES = { blue: 'Blue', green: 'Green', orange: 'Orange', pink: 'Pink', purple: 'Purple', teal: 'Teal', yellow: 'Yellow', grey: 'Grey' };
const groupColorName = (c) => i18n.tr(GROUP_COLOR_NAMES[c] || 'Group');

function item(iconName, label, onClick, opts = {}) {
  const lead = opts.dot ? h('span', { class: `gdot g-${opts.dot}` }) : iconName ? icon(iconName) : h('span', { style: { width: '16px' } });
  const el = h('button', { class: `item${opts.danger ? ' danger' : ''}` }, lead, h('span', { class: 'label', text: label }), opts.kbd ? h('span', { class: 'kbd', text: opts.kbd }) : null);
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
    d.private ? null : item('sparkle', 'New space', () => mizu.send('space_create', {})),
    sep(),
    h('div', { class: 'zoom-row' }, h('span', { class: 'label', text: 'Zoom' }), zoomBtn('zoom-out', -1, 'Zoom out (Ctrl+−)'), zoomVal, zoomBtn('zoom-in', 1, 'Zoom in (Ctrl++)'), fs),
    sep(),
    item('history', 'History', () => open('mizu://history/'), { kbd: 'Ctrl+H' }),
    item('download', 'Downloads', () => open('mizu://downloads/'), { kbd: 'Ctrl+J' }),
    item('bookmark', 'Bookmarks', () => open('mizu://bookmarks/'), { kbd: 'Ctrl+Shift+O' }),
    item('key', 'Passwords', () => open('mizu://passwords/')),
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
    ...splitItems(d),
    ...spaceItems(d),
    !d.pinned ? sep() : null,
    !d.pinned && d.group == null ? item('folder', 'Add to new group', () => mizu.send('group_create', { tab: id })) : null,
    ...(!d.pinned ? (d.groups || []).filter((g) => g.id !== d.group).slice(0, 6).map((g) => item(null, `Add to ${g.name || groupColorName(g.color)}`, () => mizu.send('group_set', { tab: id, group: g.id }), { dot: g.color })) : []),
    d.group != null ? item('close', 'Remove from group', () => mizu.send('group_set', { tab: id, group: null })) : null,
    !d.pinned ? sep() : null,
    d.away ? item('home', 'Back to pinned page', () => mizu.send('pinned_home', { id })) : null,
    d.away ? item(null, 'Pin this page instead', () => mizu.send('pinned_home', { id, reset: true })) : null,
    item(d.muted ? 'volume' : 'mute', d.muted ? 'Unmute site' : 'Mute tab', () => mizu.send('mute_tab', { id, muted: !d.muted })),
    sep(),
    !d.active && !d.unloaded ? item('pause', 'Unload tab', () => mizu.send('discard_tab', { id })) : null,
    d.others > 0 ? item(null, 'Unload other tabs', () => mizu.send('discard_tab', { id, others: true })) : null,
    sep(),
    item('close', 'Close tab', () => mizu.send('close_tab', { id }), { kbd: 'Ctrl+W' }),
    d.kids > 0 ? item(null, 'Close tab and the tabs opened from it', () => mizu.send('close_branch', { id })) : null,
    d.count > 1 ? item(null, 'Close other tabs', () => mizu.send('close_other_tabs', { id })) : null,
    d.count > 1 ? item(null, 'Close tabs to the right', () => mizu.send('close_tabs_right', { id })) : null,
    d.closed ? item('history', 'Reopen closed tab', () => mizu.send('reopen_closed_tab'), { kbd: 'Ctrl+Shift+T' }) : null,
  );
}

// ── Bookmarks bar item menu ──────────────────────────────────
function renderBookmarkMenu(d) {
  const open = (disposition) => mizu.send('open_url', { url: d.url, disposition });
  const remove = () => mizu.request('bookmark_delete', { id: d.id }).catch(() => {});
  if (d.folder) {
    return h('div', { class: 'menu', style: { width: '240px' } },
      item('bookmark', 'Open bookmarks', () => mizu.send('open_url', { url: 'mizu://bookmarks/', disposition: 'new_tab' })),
      sep(),
      item('trash', 'Delete folder', remove, { danger: true }));
  }
  return h('div', { class: 'menu', style: { width: '250px' } },
    item('external', 'Open in new tab', () => open('background')),
    item('window', 'Open in new window', () => open('new_window')),
    d.private ? null : item('private', 'Open in private window', () => open('private_window')),
    sep(),
    item('copy', 'Copy link', () => navigator.clipboard.writeText(d.url).catch(() => {})),
    item('edit', 'Edit in bookmarks…', () => mizu.send('open_url', { url: 'mizu://bookmarks/', disposition: 'new_tab' })),
    sep(),
    item('trash', 'Delete', remove, { danger: true }));
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
  else if (d.state === 'completed' && d.exists) acts.append(...(d.viewable ? [act('window', 'Open in Windows app', 'open_system')] : []), act('more', 'Open with…', 'open_with'), act('folder', 'Show in folder', 'show'));
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
  const row = h('div', { class: `dl${d.state === 'completed' && d.exists ? ' openable' : ''}`, title: d.state === 'completed' && d.exists ? 'Open' : '' }, fileBadge(d.file || d.url), meta, acts);
  if (d.state === 'completed' && d.exists) row.addEventListener('click', () => mizu.send('download_action', { id: d.id, action: 'open' }));
  return row;
}

function renderDownloads(items) {
  const list = (items || []).slice(0, 7);
  const folder = h('button', { class: 'icon-btn', title: 'Open downloads folder' }, icon('folder'));
  folder.addEventListener('click', () => {
    mizu.send('downloads_open_folder');
    close();
  });
  const active = (items || []).filter((d) => d.state === 'in_progress').length;
  const panel = h('div', { class: 'panel dl-panel' }, h('h2', null, h('span', { class: 'grow', text: 'Downloads' }), active ? h('span', { class: 'pill', text: `${active} active` }) : null, folder));
  if (!list.length) panel.append(h('div', { class: 'empty' }, icon('download', { size: 22 }), h('div', { text: 'Nothing downloaded yet.' })));
  for (const d of list) panel.append(downloadRow(d, true));
  const all = h('button', { class: 'link', text: 'All downloads' });
  all.addEventListener('click', () => open('mizu://downloads/'));
  panel.append(h('div', { class: 'footer' }, all));
  return panel;
}

// ── Music and video ──────────────────────────────────────────
function fmtTime(s) {
  s = Math.max(0, Math.floor(s || 0));
  const m = Math.floor(s / 60);
  const h = Math.floor(m / 60);
  const ss = String(s % 60).padStart(2, '0');
  return h ? `${h}:${String(m % 60).padStart(2, '0')}:${ss}` : `${m}:${ss}`;
}
function mediaCard(m, big) {
  const send = (action, value) => mizu.send('media_control', { tab: m.tab, action, value });
  const art = m.artwork
    ? h('div', { class: 'mh-art', style: { backgroundImage: `url("${m.artwork.replace(/"/g, '%22')}")` } })
    : h('div', { class: 'mh-art none' }, icon(m.video ? 'media' : 'music', { size: 22 }));
  const title = m.title || m.page_title || m.site;
  const sub = [m.artist, m.album].filter(Boolean).join(' — ') || m.site;
  const btn = (ic, label, action, cls = '') => {
    const b = h('button', { class: `mh-btn ${cls}`, title: label }, icon(ic, { size: cls.includes('main') ? 18 : 15 }));
    b.addEventListener('click', (e) => {
      e.stopPropagation();
      send(action);
    });
    return b;
  };
  const vol = h('input', { type: 'range', min: '0', max: '100', value: String(Math.round((m.muted ? 0 : m.volume) * 100)), class: 'mh-vol', title: 'Volume (remembered for this site)' });
  vol.addEventListener('input', () => send('volume', Number(vol.value) / 100));
  const go = h('button', { class: 'mh-link', title: 'Go to tab' }, h('span', { translate: 'no', text: m.site }), icon('chevron-right', { size: 12 }));
  go.addEventListener('click', () => {
    send('focus');
    close();
  });
  const progress = m.duration > 0 ? h('div', { class: 'mh-progress' }, h('div', { style: { width: `${Math.min(100, (100 * m.position) / m.duration)}%` } })) : null;
  const time = m.duration > 0 ? h('span', { class: 'mh-time', text: `${fmtTime(m.position)} / ${fmtTime(m.duration)}` }) : null;
  return h('div', { class: `mh-card${m.playing ? ' playing' : ''}${big ? ' big' : ''}` },
    art,
    h('div', { class: 'mh-meta' },
      h('div', { class: 'mh-title', translate: 'no', text: title, title }),
      h('div', { class: 'mh-sub', translate: 'no', text: sub }),
      progress,
      h('div', { class: 'mh-row' },
        btn('skip-back', 'Previous track', 'prev'),
        btn(m.playing ? 'pause' : 'play', m.playing ? 'Pause' : 'Play', 'toggle', 'main'),
        btn('skip-forward', 'Next track', 'next'),
        h('span', { class: 'mh-vol-wrap' }, icon(m.muted || m.volume === 0 ? 'mute' : 'volume', { size: 14 }), vol),
      ),
      h('div', { class: 'mh-foot' }, go, time)));
}
function renderMedia(d) {
  const items = d.items || [];
  const panel = h('div', { class: 'panel mh-panel' }, h('h2', null, icon('music', { size: 18 }), h('span', { class: 'grow', text: 'Music and video' })));
  if (!items.length) panel.append(h('div', { class: 'empty' }, icon('music', { size: 22 }), h('div', { text: 'Nothing is playing.' })));
  items.slice(0, 6).forEach((m, i) => panel.append(mediaCard(m, i === 0)));
  const settings = h('button', { class: 'link', text: 'Media settings' });
  settings.addEventListener('click', () => open('mizu://settings/#media'));
  panel.append(h('div', { class: 'footer' }, settings));
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
  if (!d.private && /^https?:/.test(d.url || '')) panel.append(siteCookies());
  const settings = h('button', { class: 'link', text: 'Site settings' });
  settings.addEventListener('click', () => open(d.host && /^https?:/.test(d.url || '') ? `mizu://site/?host=${encodeURIComponent(d.host)}` : 'mizu://settings/#privacy'));
  panel.append(h('div', { class: 'footer' }, settings));
  return panel;
}

// Cookies the site can read, with one button that signs you out of it (cookies + storage).
function siteCookies() {
  const label = h('span', { class: 'k' }, icon('cookie', { size: 14 }), h('span', { style: { marginLeft: '6px' }, text: 'Cookies…' }));
  const list = h('div', { class: 'cookie-list muted' });
  const clear = h('button', { class: 'link', text: 'Clear site data' });
  clear.disabled = true;
  const box = h('div', { class: 'rows' }, h('div', { class: 'row' }, label, clear), list);
  let open = false;
  label.style.cursor = 'pointer';
  label.addEventListener('click', () => {
    open = !open;
    list.style.display = open ? 'block' : 'none';
    reportSize();
  });
  list.style.display = 'none';
  mizu.request('site_cookies', {}).then((r) => {
    const cookies = r?.cookies || [];
    label.lastChild.textContent = cookies.length === 1 ? '1 cookie' : `${cookies.length} cookies`;
    clear.disabled = false;
    list.replaceChildren(...cookies.slice(0, 40).map((c) => h('div', { translate: 'no', text: `${c.name} · ${c.domain}${c.session ? ' · session' : ''}` })));
    reportSize();
  });
  clear.addEventListener('click', () => {
    clear.disabled = true;
    mizu.request('site_data_clear', {}).then(() => close());
  });
  return box;
}

// Split view entries of the tab menu.
function splitItems(d) {
  const id = d.id;
  const inSplit = Array.isArray(d.split) && d.split.includes(id);
  if (inSplit) {
    return [
      item('swap', 'Swap sides', () => mizu.send('split_swap')),
      item('close', 'Close split view', () => mizu.send('split_close')),
    ];
  }
  if (d.active) return [item('split', 'Split with a new tab', () => mizu.send('split_open', { tab: id }))];
  return [item('split', 'Open in split view', () => mizu.send('split_open', { tab: id }))];
}

// "Move to space" entries of the tab menu.
const spaceLabel = (s) => s.name || (s.id === 0 ? i18n.tr('Main') : i18n.tr(`Space ${s.id + 1}`));
function spaceItems(d) {
  const others = (d.spaces || []).filter((s) => s.id !== d.space);
  return [
    ...others.slice(0, 6).map((s) => item(s.icon || 'home', i18n.tr(`Move to ${spaceLabel(s)}`), () => mizu.send('tab_to_space', { tab: d.id, space: s.id }))),
    item('plus', 'Move to a new space', () => mizu.send('space_create', { tab: d.id })),
  ];
}

// ── Space menu ───────────────────────────────────────────────
function renderSpaceMenu(d) {
  const spaces = d.spaces || [];
  const cur = spaces.find((s) => s.id === d.active);
  if (!cur) return h('div', { class: 'menu' }, h('div', { class: 'empty', text: 'This space is gone.' }));
  const list = h('div', { class: 'sm-list' }, ...spaces.map((s) => {
    const b = h('button', { class: `item sm-row g-${s.color}${s.id === d.active ? ' on' : ''}` },
      h('span', { class: 'sm-icon' }, icon(s.icon || 'home')),
      h('span', { class: 'label', text: spaceLabel(s) }),
      h('span', { class: 'kbd', text: String(s.count) }));
    b.addEventListener('click', () => {
      if (s.id !== d.active) mizu.send('space_switch', { id: s.id });
      close();
    });
    return b;
  }));
  const send = (patch) => mizu.send('space_update', { id: cur.id, ...patch });
  const name = h('input', { type: 'text', value: cur.name || '', placeholder: spaceLabel(cur), maxlength: '30' });
  name.addEventListener('input', () => send({ name: name.value }));
  name.addEventListener('keydown', (e) => e.key === 'Enter' && close());
  const colors = h('div', { class: 'gcolors' }, ...(d.colors || []).map((c) => {
    const b = h('button', { class: `gdot big g-${c}${c === cur.color ? ' on' : ''}`, title: groupColorName(c) });
    b.addEventListener('click', () => {
      for (const x of colors.children) x.classList.toggle('on', x === b);
      for (const x of icons.children) x.className = x.className.replace(/g-\w+/, `g-${c}`);
      send({ color: c });
    });
    return b;
  }));
  const icons = h('div', { class: 'sm-icons' }, ...(d.icons || []).map((n) => {
    const b = h('button', { class: `sm-ico g-${cur.color}${n === cur.icon ? ' on' : ''}`, title: n }, icon(n));
    b.addEventListener('click', () => {
      for (const x of icons.children) x.classList.toggle('on', x === b);
      send({ icon: n });
    });
    return b;
  }));
  const others = spaces.filter((s) => s.id !== cur.id);
  return h('div', { class: 'menu gmenu smenu', style: { width: '280px' } },
    list,
    sep(),
    h('div', { class: 'gm-head' }, name, colors, icons),
    sep(),
    item('plus', 'New space', () => mizu.send('space_create', {})),
    others.length ? item('trash', i18n.tr(`Delete space (tabs move to ${spaceLabel(others[Math.max(0, spaces.indexOf(cur) - 1)] || others[0])})`), () => mizu.send('space_delete', { id: cur.id }), { danger: true }) : null);
}

// ── Tab group menu ───────────────────────────────────────────
function renderGroupMenu(d) {
  const g = d.group;
  if (!g) return h('div', { class: 'menu' }, h('div', { class: 'empty', text: 'This group is gone.' }));
  const name = h('input', { type: 'text', value: g.name || '', placeholder: 'Name this group', maxlength: '40' });
  const send = (patch) => mizu.send('group_update', { id: g.id, ...patch });
  name.addEventListener('input', () => send({ name: name.value }));
  name.addEventListener('keydown', (e) => e.key === 'Enter' && close());
  const colors = h('div', { class: 'gcolors' }, ...(d.colors || []).map((c) => {
    const b = h('button', { class: `gdot big g-${c}${c === g.color ? ' on' : ''}`, title: groupColorName(c) });
    b.addEventListener('click', () => {
      for (const x of colors.children) x.classList.toggle('on', x === b);
      send({ color: c });
    });
    return b;
  }));
  setTimeout(() => {
    name.focus();
    name.select();
  }, 30);
  return h('div', { class: 'menu gmenu', style: { width: '260px' } },
    h('div', { class: 'gm-head' }, name, colors),
    sep(),
    item(g.collapsed ? 'chevron-down' : 'chevron-right', g.collapsed ? 'Expand group' : 'Collapse group', () => send({ collapsed: !g.collapsed })),
    item('folder', 'Ungroup', () => mizu.send('group_ungroup', { id: g.id })),
    item('close', d.count === 1 ? 'Close the tab' : `Close ${d.count} tabs`, () => mizu.send('group_close', { id: g.id }), { danger: true }));
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
    if (parent && parent !== bm.parent_id) await mizu.request('bookmark_move', { id: bm.id, parent, index: 1e6, remember: true });
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
const SUG_ICONS = {
  suggest: 'search', navigate: 'globe', search: 'search', history: 'history', bookmark: 'star', internal: 'drop', tab: 'tabs', calc: 'sliders', command: 'sparkle' };
function renderOmnibox(d) {
  const box = h('div', { class: 'omni', style: { width: `${Math.max(320, Math.round(d.width || 560)) - 12}px` } });
  (d.items || []).forEach((s, i) => {
    const hint = s.kind === 'tab' || s.kind === 'calc';
    const row = h('div', { class: `sug k-${s.kind}${i === d.selected ? ' sel' : ''}` }, icon(SUG_ICONS[s.kind] || 'globe'), h('span', { class: 't', text: s.title, translate: s.kind === 'internal' || s.kind === 'command' ? null : 'no' }), s.display && s.display !== s.title ? h('span', { class: 'd', text: s.kind === 'calc' ? `${s.display} · Enter to copy` : s.display, translate: hint && s.kind !== 'tab' ? 'no' : null }) : null);
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

// ── Passwords ────────────────────────────────────────────────
function siteBadge(host) {
  const s = host || '?';
  let hash = 0;
  for (const c of s) hash = (hash * 31 + c.charCodeAt(0)) | 0;
  const hue = Math.abs(hash) % 360;
  return h('div', { class: 'site-badge', text: s.charAt(0).toUpperCase(), style: { background: `hsl(${hue} 60% 50% / 0.18)`, color: `hsl(${hue} 55% 55%)` } });
}

function renderSavePassword(d) {
  const user = h('input', { type: 'text', value: d.username || '', placeholder: 'Username or email', spellcheck: 'false' });
  const pass = h('input', { type: 'password', value: d.password || '', readonly: true });
  const eye = h('button', { class: 'mini eye', title: 'Show password' }, icon('eye'));
  eye.addEventListener('click', () => {
    const show = pass.type === 'password';
    pass.type = show ? 'text' : 'password';
    eye.replaceChildren(icon(show ? 'eye-off' : 'eye'));
    eye.title = show ? 'Hide password' : 'Show password';
  });
  const respond = (action) => mizu.send('password_prompt_response', { action, username: user.value });
  const never = h('button', { class: 'link subtle', text: 'Never for this site' });
  never.addEventListener('click', () => respond('never'));
  const later = h('button', { class: 'btn', text: 'Not now' });
  later.addEventListener('click', () => respond('dismiss'));
  const save = h('button', { class: 'btn primary', text: d.update ? 'Update' : 'Save' });
  save.addEventListener('click', () => respond('save'));
  user.addEventListener('keydown', (e) => e.key === 'Enter' && respond('save'));
  const close = h('button', { class: 'mini', title: 'Close' }, icon('close'));
  close.addEventListener('click', () => respond('dismiss'));
  setTimeout(() => save.focus(), 40);
  return h('div', { class: 'pwsave' },
    h('div', { class: 'pw-head' }, siteBadge(d.host),
      h('div', { class: 'pw-title' }, h('div', { class: 'ttl', text: d.update ? 'Update password?' : 'Save password?' }), h('div', { class: 'muted', translate: 'no', text: d.host })),
      close),
    h('label', { class: 'pw-field' }, h('span', { text: 'Username' }), h('div', { class: 'pw-input' }, icon('user'), user)),
    h('label', { class: 'pw-field' }, h('span', { text: 'Password' }), h('div', { class: 'pw-input' }, icon('key'), pass, eye)),
    h('div', { class: 'pw-note' }, icon('lock', { size: 13 }), h('span', { text: 'Encrypted with your Windows account. Only you can read it on this PC.' })),
    h('div', { class: 'pw-actions' }, never, h('div', { class: 'grow' }), later, save),
  );
}

function renderLogins(d) {
  const panel = h('div', { class: 'panel', style: { width: '320px' } },
    h('h2', null, siteBadge(d.host), h('span', { class: 'grow', translate: 'no', text: d.host })));
  for (const l of d.logins || []) {
    const row = h('button', { class: 'item acct' }, icon('user'), h('span', { class: 'label', translate: 'no', text: l.username || '(no username)' }), h('span', { class: 'kbd', text: 'Fill' }));
    row.addEventListener('click', () => mizu.request('password_fill', { id: l.id }).catch(() => {}));
    panel.append(row);
  }
  const manage = h('button', { class: 'link', text: 'Manage passwords' });
  manage.addEventListener('click', () => open('mizu://passwords/'));
  panel.append(h('div', { class: 'footer' }, manage));
  return panel;
}

// ── Confirmation (closing many tabs) ─────────────────────────
function renderConfirm(d) {
  const skip = h('input', { type: 'checkbox' });
  const cancel = h('button', { class: 'btn', text: d.cancel || 'Cancel' });
  cancel.addEventListener('click', () => mizu.send('confirm_response', { accepted: false }));
  const ok = h('button', { class: d.primary ? 'btn primary' : 'btn danger', text: d.confirm || 'OK' });
  ok.addEventListener('click', () => mizu.send('confirm_response', { accepted: true, dont_ask_again: skip.checked }));
  setTimeout(() => ok.focus(), 30);
  return h(
    'div',
    { class: 'confirm' },
    h('div', { class: 'q' }, h('div', { class: 'ic' }, icon(d.icon || 'tabs', { size: 20 })), h('div', null, h('div', { class: 'ct', text: d.title }), h('div', { class: 'muted', text: d.message }))),
    d.can_skip ? h('label', { class: 'check' }, skip, d.skip_label || "Don't ask again") : null,
    h('div', { class: 'footer' }, cancel, ok),
  );
}

const RENDERERS = {
  confirm: renderConfirm,
  save_password: renderSavePassword,
  logins: renderLogins,
  menu: renderMenu,
  tab_menu: renderTabMenu,
  group_menu: renderGroupMenu,
  space_menu: renderSpaceMenu,
  bookmark_menu: renderBookmarkMenu,
  downloads: renderDownloads,
  media: renderMedia,
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

mizu.on('media', (m) => {
  if (kind !== 'media') return;
  // Keep a slider being dragged: only redraw when the pointer is not on one.
  if (document.querySelector('.mh-vol:active')) return;
  root.replaceChildren(renderMedia({ items: m.items }));
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
