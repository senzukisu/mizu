// Browser chrome: tab strip, toolbar, omnibox and bookmarks bar.
'use strict';

const $ = (id) => document.getElementById(id);
const els = {
  app: $('app'),
  tabs: $('tabs'),
  address: $('address'),
  omnibox: $('omnibox'),
  site: $('site'),
  star: $('star'),
  shield: $('shield-btn'),
  shieldCount: $('shield-count'),
  progress: $('progress'),
  downloads: $('downloads'),
  ring: document.querySelector('#dl-ring circle'),
  bookmarks: $('bookmarks'),
  toolbar: $('toolbar'),
  toast: $('toast'),
};

// The same document renders two surfaces: the top chrome and, for vertical tabs, the
// sidebar (loaded as mizu://chrome/?sidebar).
const SIDEBAR = new URLSearchParams(location.search).has('sidebar');
const VERTICAL = SIDEBAR;
let sideLayout = false; // top chrome: tabs are shown in the sidebar instead

let state = null;
let settings = null;
const favicons = new Map();
const tabEls = new Map();

if (SIDEBAR) {
  document.body.classList.add('sidebar-surface');
  const newTab = h('button', { class: 'vt-new', title: 'New tab (Ctrl+T)' }, h('span', { class: 'vt-plus' }, icon('plus')), h('span', { class: 'vt-label', text: 'New tab' }));
  newTab.addEventListener('click', () => mizu.send('new_tab'));
  const collapse = h('button', { class: 'vt-collapse', title: 'Collapse sidebar' }, icon('chevron-right'));
  collapse.addEventListener('click', () => mizu.send('sidebar_toggle'));
  // Tabs and the "New tab" button scroll together, so the button always sits under the last tab.
  const list = h('div', { id: 'vt-list' }, els.tabs, newTab);
  list.addEventListener('dblclick', (e) => {
    if (e.target === list || e.target === els.tabs) mizu.send('new_tab');
  });
  // Drag the right edge to resize; double-click resets the width.
  const grip = h('div', { class: 'vt-grip', title: 'Drag to resize' });
  grip.addEventListener('pointerdown', (e) => {
    if (e.button !== 0 || document.body.classList.contains('collapsed')) return;
    e.preventDefault();
    grip.setPointerCapture(e.pointerId);
    grip.classList.add('active');
    // Screen coordinates stay valid while the view itself changes size.
    const startX = e.screenX;
    const startW = window.innerWidth;
    let pending = 0;
    let last = startW;
    const move = (ev) => {
      last = Math.max(160, Math.min(520, startW + (ev.screenX - startX) / (window.devicePixelRatio || 1) * (window.devicePixelRatio || 1)));
      if (pending) return;
      pending = requestAnimationFrame(() => {
        pending = 0;
        mizu.send('sidebar_resize', { width: last, commit: false });
      });
    };
    const up = () => {
      grip.removeEventListener('pointermove', move);
      grip.removeEventListener('pointerup', up);
      grip.removeEventListener('pointercancel', up);
      grip.classList.remove('active');
      mizu.send('sidebar_resize', { width: last, commit: true });
    };
    grip.addEventListener('pointermove', move);
    grip.addEventListener('pointerup', up);
    grip.addEventListener('pointercancel', up);
  });
  grip.addEventListener('dblclick', () => mizu.send('sidebar_resize', { width: 252, commit: true }));
  const count = h('span', { class: 'vt-count' });
  // The top-chrome parts stay in the DOM (listeners below expect them) but are hidden by CSS.
  els.app.append(h('div', { id: 'vt' }, list, h('div', { class: 'vt-foot' }, count, collapse)), grip);
}

// ── Static buttons ───────────────────────────────────────────
$('new-tab').append(icon('plus'));
$('back').append(icon('back'));
$('forward').append(icon('forward'));
$('reload').append(icon('reload'));
$('home').append(icon('home'));
els.star.append(icon('star'));
els.shield.prepend(icon('shield'));
els.downloads.append(icon('download'));
$('menu').append(icon('menu'));
$('cap-min').append(icon('minimize'));
$('cap-max').append(icon('maximize'));
$('cap-close').append(icon('close'));

const rect = (el) => {
  const r = el.getBoundingClientRect();
  return { x: r.left, y: r.top, width: r.width, height: r.height };
};
const flyout = (kind, el, extra = {}) => mizu.send('show_flyout', { kind, anchor: rect(el), ...extra });

$('new-tab').addEventListener('click', () => mizu.send('new_tab'));
$('back').addEventListener('click', () => mizu.send('back'));
$('forward').addEventListener('click', () => mizu.send('forward'));
$('reload').addEventListener('click', (e) => {
  if (state?.nav?.loading) mizu.send('stop');
  else mizu.send('reload', { bypass_cache: e.shiftKey || e.ctrlKey });
});
$('home').addEventListener('click', () => mizu.send('home'));
$('menu').addEventListener('click', () => flyout('menu', $('menu')));
els.downloads.addEventListener('click', () => flyout('downloads', els.downloads));
els.shield.addEventListener('click', () => flyout('shield', els.shield));
els.site.addEventListener('click', () => flyout('site_info', els.site));
const pwBtn = $('pw-btn');
pwBtn.append(icon('key'));
pwBtn.addEventListener('click', () => flyout('logins', pwBtn));
const extInstall = $('ext-install');
extInstall.append(icon('puzzle'), h('span', { text: 'Add to Mizu' }));
extInstall.addEventListener('click', () => {
  extInstall.disabled = true;
  toast('Installing extension…');
  mizu
    .request('extension_install_from_store', { url: state?.nav?.url || '' })
    .then((r) => toast(`${r.name} added to Mizu`))
    .catch((e) => toast(e.message))
    .finally(() => (extInstall.disabled = false));
});
els.star.addEventListener('click', () => {
  if (!state?.nav?.bookmarked) mizu.send('toggle_bookmark');
  flyout('bookmark', els.star);
});
$('cap-min').addEventListener('click', () => mizu.send('window', { action: 'minimize' }));
$('cap-max').addEventListener('click', () => mizu.send('window', { action: 'maximize' }));
$('cap-close').addEventListener('click', () => mizu.send('window', { action: 'close' }));
document.querySelector('.drag-fill').addEventListener('dblclick', () => mizu.send('new_tab'));
els.tabs.addEventListener('dblclick', (e) => {
  if (e.target === els.tabs) mizu.send('new_tab');
});

// ── Metrics: tell the browser how tall the chrome is and where the omnibox sits ──
let lastMetrics = '';
function reportMetrics() {
  if (SIDEBAR) return;
  const m = { height: Math.ceil(els.app.getBoundingClientRect().height), omnibox: rect(els.omnibox) };
  const key = JSON.stringify(m);
  if (key !== lastMetrics) {
    lastMetrics = key;
    mizu.send('chrome_metrics', m);
  }
}
new ResizeObserver(reportMetrics).observe(els.app);
new ResizeObserver(reportMetrics).observe(els.omnibox);

// ── Toasts ──────────────────────────────────────────────────
let chromeToastTimer = 0;
function toast(message) {
  els.toast.textContent = message;
  els.toast.classList.add('show');
  clearTimeout(chromeToastTimer);
  chromeToastTimer = setTimeout(() => els.toast.classList.remove('show'), 2600);
}

// ── Tabs ────────────────────────────────────────────────────
function favNode(t) {
  if (t.loading) return h('div', { class: 'spinner' });
  if (t.crashed) return icon('warning');
  if (t.internal) return icon('drop');
  const data = favicons.get(t.id);
  if (data) return h('img', { src: data, alt: '' });
  const letter = (hostOf(t.url) || t.title || '?').charAt(0).toUpperCase();
  return t.url.startsWith('http') ? h('div', { class: 'letter', text: letter }) : icon('globe');
}

// Tabs are built once and then updated in place. Rebuilding children on every update
// (titles, favicons, loading) would replace the close button between mouse-down and
// mouse-up and lose clicks.
function buildTab(t) {
  const el = h('div', { class: 'tab', role: 'tab', dataset: { id: t.id } });
  const fav = h('div', { class: 'fav' });
  const title = h('div', { class: 'title' });
  const audio = h('button', { class: 'audio', hidden: true });
  const close = h('button', { class: 'close', title: 'Close tab (Ctrl+W)' }, icon('close'));
  el.append(fav, title, audio, close);
  el._parts = { fav, title, audio, close, favKey: '', audioKey: '' };

  for (const b of [audio, close]) b.addEventListener('pointerdown', (e) => e.stopPropagation());
  audio.addEventListener('click', (e) => {
    e.stopPropagation();
    mizu.send('mute_tab', { id: Number(el.dataset.id), muted: !el._muted });
  });
  close.addEventListener('click', (e) => {
    e.stopPropagation();
    closeTab(el);
  });
  el.addEventListener('pointerdown', (e) => onTabPointerDown(e, el));
  el.addEventListener('auxclick', (e) => {
    if (e.button === 1) {
      e.preventDefault();
      closeTab(el);
    }
  });
  el.addEventListener('mousedown', (e) => {
    if (e.button === 1) e.preventDefault();
  });
  el.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    mizu.send('show_flyout', {
      kind: 'tab_menu',
      anchor: { x: e.clientX, y: e.clientY, width: 1, height: 1 },
      tab_id: Number(el.dataset.id),
    });
  });
  observeTab(el);
  return el;
}

function fillTab(el, t) {
  const p = el._parts;
  const favKey = [t.loading, t.crashed, t.internal, favicons.get(t.id) || '', hostOf(t.url)].join('|');
  if (p.favKey !== favKey) {
    p.favKey = favKey;
    p.fav.replaceChildren(favNode(t));
  }
  const text = t.title || 'New Tab';
  if (t.internal || !t.title) p.title.removeAttribute('translate');
  else p.title.setAttribute('translate', 'no');
  if (p.title.textContent !== text) p.title.textContent = text;
  const tip = t.title + (t.url && !t.internal ? `\n${t.url}` : '');
  if (el.title !== tip) el.title = tip;
  el._muted = !!t.muted;
  const audioKey = t.audible || t.muted ? (t.muted ? 'mute' : 'volume') : '';
  if (p.audioKey !== audioKey) {
    p.audioKey = audioKey;
    p.audio.hidden = !audioKey;
    if (audioKey) {
      p.audio.replaceChildren(icon(audioKey));
      p.audio.title = t.muted ? 'Unmute tab' : 'Mute tab';
    }
  }
}

// ── Tab animations (FLIP) ───────────────────────────────────
function motionMs(base) {
  if (document.documentElement.dataset.motion === 'off') return 0;
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return 0;
  return base;
}
const EASE_OUT = 'cubic-bezier(0.2, 0.8, 0.2, 1)';

// Records tab positions, runs `mutate`, then animates every tab from its old position.
function flip(mutate) {
  const dur = motionMs(220);
  const before = new Map();
  const pos = (el) => (VERTICAL ? el.getBoundingClientRect().top : el.getBoundingClientRect().left);
  if (dur) for (const el of els.tabs.children) if (!el._out) before.set(el, pos(el));
  const follower = VERTICAL && document.querySelector('#vt-list > .vt-new');
  if (dur && follower) before.set(follower, pos(follower));
  mutate();
  if (!dur) return;
  const axis = VERTICAL ? 'translateY' : 'translateX';
  for (const [el, old] of before) {
    if (!el.isConnected || el._out) continue;
    const d = old - pos(el);
    if (Math.abs(d) < 0.5) continue;
    el.animate([{ transform: `${axis}(${d}px)` }, { transform: `${axis}(0)` }], { duration: dur, easing: EASE_OUT });
  }
}

// Takes a tab out of the layout immediately and fades it out where it stood.
function ghostOut(el) {
  if (el._out) return;
  el._out = true;
  const strip = els.tabs.getBoundingClientRect();
  const r = el.getBoundingClientRect();
  el.classList.add('ghost');
  el.classList.remove('active');
  Object.assign(el.style, {
    left: `${r.left - strip.left}px`,
    top: VERTICAL ? `${r.top - strip.top + els.tabs.scrollTop}px` : '',
    bottom: VERTICAL ? 'auto' : '',
    width: `${r.width}px`,
    height: `${r.height}px`,
  });
  const dur = motionMs(160);
  if (!dur) return el.remove();
  el.animate(
    [{ opacity: 1, transform: 'scale(1)' }, { opacity: 0, transform: VERTICAL ? 'translateX(-12px) scale(0.96)' : 'scale(0.92) translateY(2px)' }],
    { duration: dur, easing: 'ease-in', fill: 'forwards' },
  ).onfinish = () => el.remove();
}

function animateIn(el) {
  const dur = motionMs(240);
  if (!dur) return;
  el.animate(
    [{ opacity: 0, transform: VERTICAL ? 'translateX(-14px)' : 'translateY(8px) scale(0.94)' }, { opacity: 1, transform: 'none' }],
    { duration: dur, easing: 'cubic-bezier(0.34, 1.25, 0.5, 1)' },
  );
}

function closeTab(el) {
  const id = Number(el.dataset.id);
  tabEls.delete(id);
  flip(() => ghostOut(el));
  mizu.send('close_tab', { id });
}

let lastOrder = '';
function renderTabs() {
  const tabs = state.tabs;
  const order = tabs.map((t) => t.id).join(',');
  const structural = order !== lastOrder;
  lastOrder = order;

  const update = () => {
    const seen = new Set();
    const fresh = [];
    for (const t of tabs) {
      seen.add(t.id);
      let el = tabEls.get(t.id);
      if (!el) {
        el = buildTab(t);
        tabEls.set(t.id, el);
        if (rendered) fresh.push(el);
      }
      el.classList.toggle('active', t.id === state.active);
      el.classList.toggle('pinned', !!t.pinned);
      el.classList.toggle('sleeping', !!t.sleeping);
      el.setAttribute('aria-selected', t.id === state.active ? 'true' : 'false');
      fillTab(el, t);
    }
    for (const [id, el] of tabEls) {
      if (!seen.has(id)) {
        tabEls.delete(id);
        ghostOut(el);
      }
    }
    if (structural) {
      // One linear pass: append live tabs in order (ghosts are absolutely positioned).
      for (const t of tabs) els.tabs.append(tabEls.get(t.id));
    }
    return fresh;
  };

  if (structural && rendered) {
    let fresh = [];
    flip(() => (fresh = update()));
    fresh.forEach(animateIn);
  } else {
    update();
  }
  rendered = true;
}
let rendered = false;

// Narrow-tab layout follows the strip's size without forcing layout on every state update.
const narrowObserver = new ResizeObserver((entries) => {
  for (const e of entries) e.target.classList.toggle('narrow', e.contentRect.width < 60);
});
const observeTab = (el) => narrowObserver.observe(el);

// Drag to reorder (within the strip). Pinned and unpinned tabs stay in their own groups;
// the browser enforces that too.
function onTabPointerDown(e, el) {
  if (e.button !== 0) return;
  const id = Number(el.dataset.id);
  const coord = (ev) => (VERTICAL ? ev.clientY : ev.clientX);
  const startX = coord(e);
  const siblings = [...els.tabs.children].filter((c) => !c._out);
  const from = siblings.indexOf(el);
  const rects = siblings.map((s) => s.getBoundingClientRect());
  let dragging = false;
  let target = from;
  el.setPointerCapture(e.pointerId);

  const move = (ev) => {
    const dx = coord(ev) - startX;
    if (!dragging && Math.abs(dx) < 5) return;
    if (!dragging) {
      dragging = true;
      el.classList.add('dragging');
    }
    const axis = VERTICAL ? 'translateY' : 'translateX';
    const start = (r) => (VERTICAL ? r.top : r.left);
    const size = (r) => (VERTICAL ? r.height : r.width);
    el.style.transform = `${axis}(${dx}px)`;
    const center = start(rects[from]) + size(rects[from]) / 2 + dx;
    target = from;
    rects.forEach((r, i) => {
      if (i < from && center < start(r) + size(r) / 2) target = Math.min(target, i);
      if (i > from && center > start(r) + size(r) / 2) target = Math.max(target, i);
    });
    const w = size(rects[from]);
    siblings.forEach((s, i) => {
      if (s === el) return;
      let shift = 0;
      if (from < target && i > from && i <= target) shift = -w;
      if (from > target && i < from && i >= target) shift = w;
      s.style.transform = shift ? `${axis}(${shift}px)` : '';
    });
  };
  const up = () => {
    el.removeEventListener('pointermove', move);
    el.removeEventListener('pointerup', up);
    el.removeEventListener('pointercancel', up);
    siblings.forEach((s) => (s.style.transform = ''));
    el.classList.remove('dragging');
    if (dragging) {
      if (target !== from) {
        els.tabs.insertBefore(el, target > from ? siblings[target].nextSibling : siblings[target]);
        mizu.send('move_tab', { id, index: target });
      }
    } else {
      mizu.send('activate_tab', { id });
    }
  };
  el.addEventListener('pointermove', move);
  el.addEventListener('pointerup', up);
  el.addEventListener('pointercancel', up);
}

// ── Omnibox ─────────────────────────────────────────────────
let suggestions = [];
let selected = -1;
let editing = false;
let blurTimer = 0;

function showUrl() {
  if (editing) return;
  const nav = state?.nav;
  els.address.value = nav ? nav.display : '';
  els.address.classList.toggle('dim-scheme', false);
}

els.address.addEventListener('focus', () => {
  clearTimeout(blurTimer);
  els.omnibox.classList.add('focused');
  requestAnimationFrame(() => els.address.select());
});
els.address.addEventListener('blur', () => {
  els.omnibox.classList.remove('focused');
  blurTimer = setTimeout(() => {
    editing = false;
    suggestions = [];
    selected = -1;
    mizu.send('omnibox_close');
    showUrl();
  }, 180);
});
els.address.addEventListener('input', () => {
  editing = true;
  selected = 0;
  mizu.send('omnibox_input', { text: els.address.value });
});
els.address.addEventListener('keydown', (e) => {
  if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
    if (!suggestions.length) return;
    e.preventDefault();
    const n = suggestions.length;
    selected = e.key === 'ArrowDown' ? (selected + 1) % n : (selected - 1 + n) % n;
    mizu.send('omnibox_highlight', { index: selected });
    return;
  }
  if (e.key === 'Enter') {
    e.preventDefault();
    const text = els.address.value.trim();
    if (!text) return;
    if (e.altKey) {
      mizu.send('open_url', { url: text, disposition: 'new_tab' });
    } else if (editing && suggestions.length && selected >= 0) {
      mizu.send('omnibox_commit', { index: selected });
    } else {
      mizu.send('navigate', { input: text });
    }
    editing = false;
    suggestions = [];
    els.address.blur();
    return;
  }
  if (e.key === 'Escape') {
    e.preventDefault();
    if (editing) {
      editing = false;
      suggestions = [];
      mizu.send('omnibox_close');
      showUrl();
      els.address.select();
    } else {
      els.address.blur();
    }
  }
});

// ── Rendering ───────────────────────────────────────────────
const SITE_ICONS = { secure: 'lock', insecure: 'warning', internal: 'drop', file: 'file', other: 'globe' };
let lastSite = '';
let wasLoading = false;

function renderNav() {
  const nav = state.nav || {};
  $('back').disabled = !nav.can_go_back;
  $('forward').disabled = !nav.can_go_forward;
  const reload = $('reload');
  const loadingIcon = nav.loading ? 'stop' : 'reload';
  if (reload.dataset.icon !== loadingIcon) {
    reload.dataset.icon = loadingIcon;
    reload.replaceChildren(icon(loadingIcon));
    reload.title = nav.loading ? 'Stop (Esc)' : 'Reload (Ctrl+R)';
  }

  const site = nav.security || 'other';
  if (site !== lastSite) {
    lastSite = site;
    els.site.className = site;
    els.site.replaceChildren(icon(SITE_ICONS[site] || 'globe'));
    els.site.title = {
      secure: 'Connection is secure',
      insecure: 'Connection is not secure',
      internal: 'Mizu page',
      file: 'Local file',
    }[site] || 'Site information';
  }

  els.star.hidden = state.private || !nav.url || nav.security === 'internal';
  els.star.classList.toggle('on', !!nav.bookmarked);
  els.star.replaceChildren(icon(nav.bookmarked ? 'star-filled' : 'star'));

  const install = $('ext-install');
  install.hidden = !nav.store_extension;
  pwBtn.hidden = !(nav.logins > 0);
  pwBtn.title = nav.logins > 1 ? `${nav.logins} saved passwords for this site` : 'Saved password for this site';

  const web = nav.security === 'secure' || nav.security === 'insecure';
  els.shield.hidden = !web;
  els.shield.classList.toggle('active', !!nav.shield);
  els.shield.classList.toggle('off', !nav.shield);
  els.shieldCount.textContent = nav.shield && nav.blocked ? String(nav.blocked > 999 ? '999+' : nav.blocked) : '';
  els.shield.title = nav.shield ? `Mizu Shield: ${nav.blocked || 0} blocked on this page` : 'Mizu Shield is off for this site';

  const zb = $('zoom-badge');
  zb.hidden = !nav.zoom || nav.zoom === 100;
  zb.textContent = `${nav.zoom}%`;

  if (nav.loading && !wasLoading) {
    els.progress.className = '';
    void els.progress.offsetWidth;
    els.progress.className = 'loading';
  } else if (!nav.loading && wasLoading) {
    els.progress.className = 'done';
  }
  wasLoading = !!nav.loading;

  if (document.activeElement !== els.address) showUrl();
  els.address.placeholder = `Search ${settings?._engine || 'the web'} or enter address`;
}

const updateBtn = $('update-btn');
updateBtn.addEventListener('click', () => mizu.request('update_install').catch((e) => toast(e.message)));
function renderUpdate() {
  const v = state.update;
  updateBtn.hidden = !v;
  if (v && updateBtn.dataset.v !== v) {
    updateBtn.dataset.v = v;
    updateBtn.replaceChildren(icon('upload', { size: 14 }), h('span', { text: 'Update' }));
    updateBtn.title = `Restart to update to Mizu ${v}`;
  }
}

function renderDownloads() {
  const d = state.downloads || {};
  els.downloads.hidden = !d.any;
  els.downloads.classList.toggle('active', d.active > 0);
  const p = d.active > 0 && d.progress >= 0 ? d.progress : d.active > 0 ? 0.08 : 0;
  els.ring.style.strokeDashoffset = String(97.4 * (1 - p));
  els.downloads.title = d.active ? `Downloads — ${d.active} in progress` : 'Downloads (Ctrl+J)';
}

function renderBookmarks() {
  const show = !state.private && settings?.appearance?.show_bookmarks_bar !== false;
  els.bookmarks.hidden = !show;
  els.toolbar.classList.toggle('no-bookmarks', !show);
  if (!show) return;
  const items = state.bookmarks || [];
  const sig = JSON.stringify(items);
  if (els.bookmarks._sig === sig) return;
  els.bookmarks._sig = sig;
  els.bookmarks.replaceChildren();
  if (!items.length) {
    els.bookmarks.append(h('span', { class: 'bm-empty', text: 'Bookmark pages with Ctrl+D or the star to see them here.' }));
    return;
  }
  for (const b of items) {
    const isFolder = b.kind === 'folder';
    const el = h('button', { class: 'bm', translate: 'no', title: isFolder ? b.title : `${b.title}\n${b.url}` }, icon(isFolder ? 'folder' : 'bookmark'), h('span', { text: b.title || b.url }));
    el.addEventListener('click', (e) => {
      if (isFolder) mizu.send('open_url', { url: 'mizu://bookmarks/', disposition: 'current' });
      else mizu.send('open_url', { url: b.url, disposition: e.ctrlKey ? 'background' : e.shiftKey ? 'new_window' : 'current' });
    });
    el.addEventListener('auxclick', (e) => {
      if (e.button === 1 && !isFolder) mizu.send('open_url', { url: b.url, disposition: 'background' });
    });
    els.bookmarks.append(el);
  }
}

function renderSidebar() {
  document.documentElement.toggleAttribute('data-private', !!state.private);
  document.body.classList.toggle('collapsed', !!settings?.appearance?.sidebar_collapsed);
  const c = document.querySelector('.vt-collapse');
  if (c) c.title = settings?.appearance?.sidebar_collapsed ? 'Expand sidebar' : 'Collapse sidebar';
  const n = document.querySelector('.vt-count');
  if (n) n.textContent = `${state.tabs.length} ${state.tabs.length === 1 ? 'tab' : 'tabs'}`;
  renderTabs();
}

function render() {
  if (!state) return;
  if (SIDEBAR) return renderSidebar();
  els.app.classList.toggle('side-tabs', sideLayout);
  if (sideLayout) {
    const active = state.tabs.find((t) => t.id === state.active);
    const wt = $('win-title');
    const text = active ? active.title || 'New Tab' : '';
    if (wt.textContent !== text) wt.textContent = text;
  }
  document.documentElement.toggleAttribute('data-private', !!state.private);
  const badge = $('private-badge');
  badge.hidden = !state.private;
  if (state.private && !badge.childElementCount) badge.append(icon('private'), h('span', { text: 'Private' }));
  $('cap-max').replaceChildren(icon(state.maximized ? 'restore' : 'maximize'));
  $('cap-max').title = state.maximized ? 'Restore' : 'Maximize';
  if (!sideLayout) renderTabs();
  renderNav();
  renderDownloads();
  renderUpdate();
  renderBookmarks();
  reportMetrics();
}

function applySettings(s) {
  settings = s;
  theme.apply(s, { private: state?.private, translucent: !SIDEBAR });
  $('home').hidden = !s.appearance?.show_home_button;
  const side = s.appearance?.tab_layout === 'side';
  if (!SIDEBAR && side !== sideLayout) {
    sideLayout = side;
    // Switching layouts: rebuild the strip from scratch next time it is shown.
    lastOrder = '';
    rendered = false;
    for (const el of tabEls.values()) el.remove();
    tabEls.clear();
  }
  if (state) render();
}

// ── Browser events ──────────────────────────────────────────
// Many state updates can arrive per frame (loading, titles, favicons): render once per frame.
let renderQueued = false;
mizu.on('state', (m) => {
  state = m.state;
  if (renderQueued) return;
  renderQueued = true;
  requestAnimationFrame(() => {
    renderQueued = false;
    render();
  });
});
mizu.on('favicon', (m) => {
  if (m.data) favicons.set(m.id, m.data);
  else favicons.delete(m.id);
  const el = tabEls.get(m.id);
  const t = state?.tabs.find((x) => x.id === m.id);
  if (el && t) fillTab(el, t);
});
mizu.on('suggestions', (m) => {
  suggestions = m.items || [];
  selected = suggestions.length ? 0 : -1;
});
mizu.on('settings', (m) => applySettings(m.settings));
mizu.on('focus_omnibox', () => {
  if (SIDEBAR) return;
  els.address.focus();
  els.address.select();
});
mizu.on('toast', (m) => !SIDEBAR && toast(m.message));
mizu.on('download_started', () => !SIDEBAR && flyout('downloads', els.downloads));

window.addEventListener('keydown', (e) => {
  // Keep browser shortcuts inside the chrome from triggering page-like behaviour.
  if (e.key === 'F5' || (e.ctrlKey && e.key.toLowerCase() === 'r')) e.preventDefault();
});
document.addEventListener('contextmenu', (e) => {
  if (e.target !== els.address) e.preventDefault();
});

mizu.request('ready').then((info) => {
  applySettings(info.settings);
  if (info.private) document.documentElement.dataset.private = '';
});
