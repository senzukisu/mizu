// New tab page: search (with engine switch), shortcut tiles with "+", Shield.
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

function tileDialog(tile, folder = null) {
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
        if (tile) await mizu.request('shortcut_update', { id: tile.id, title: name, url: u });
        else await mizu.request('shortcut_add', { title: name, url: u, folder });
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

const DRAG = 'text/x-mizu-shortcut';
const tileLabel = (s) => s.title || (s.host || hostOf(s.url) || s.url).replace(/^www\./, '');
const act = (iconName, title, fn) => {
  const b = h('button', { class: 'nt-tile-act', title }, icon(iconName, { size: 13 }));
  b.addEventListener('click', (e) => {
    e.preventDefault();
    e.stopPropagation();
    fn();
  });
  return b;
};

// The middle of a tile makes a folder (or drops into one); its edges reorder.
function dropZone(e, el) {
  const r = el.getBoundingClientRect();
  const x = (e.clientX - r.left) / r.width;
  return x > 0.25 && x < 0.75 ? 'merge' : 'order';
}

// A link tile. `folder` is the id of the folder it sits in (null at the top level).
function linkTile(s, list, folder) {
  const a = h('a', { class: 'nt-tile', href: s.url, title: s.url, translate: 'no', draggable: 'true' }, letterAvatar(s.url, s.title), h('span', { text: tileLabel(s) }));
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
  const acts = [act('edit', 'Edit', () => tileDialog(s))];
  if (folder != null) acts.push(act('export', 'Take out of the folder', () => mizu.request('shortcut_move', { id: s.id, index: 999, into: 0 }).then(load)));
  acts.push(act('close', 'Remove', () => mizu.request('shortcut_delete', { id: s.id }).then(load)));
  a.append(h('div', { class: 'nt-tile-acts' }, ...acts));
  dragTarget(a, s, list, folder);
  return a;
}

// A folder tile: up to four little icons; click to open it.
function folderTile(f, list) {
  const items = f.items || [];
  const mini = h('div', { class: 'fav nt-folder-fav' }, ...items.slice(0, 4).map((x) => letterAvatar(x.url, x.title)));
  const a = h('a', { class: 'nt-tile is-folder', href: '#', title: f.title, draggable: 'true' }, mini, h('span', { text: f.title || i18n.tr('Folder') }));
  a.addEventListener('click', (e) => {
    e.preventDefault();
    openFolder(f.id);
  });
  a.append(h('div', { class: 'nt-tile-acts' },
    act('close', 'Remove folder (keeps its shortcuts)', () => mizu.request('shortcut_delete', { id: f.id }).then(load))));
  dragTarget(a, f, list, null);
  return a;
}

function dragTarget(a, s, list, folder) {
  a.addEventListener('dragstart', (e) => {
    e.dataTransfer.setData(DRAG, JSON.stringify({ id: s.id, folder: !!s.folder }));
    e.dataTransfer.effectAllowed = 'move';
    a.classList.add('dragging');
  });
  a.addEventListener('dragend', () => a.classList.remove('dragging'));
  a.addEventListener('dragover', (e) => {
    if (!e.dataTransfer.types.includes(DRAG)) return;
    e.preventDefault();
    const merge = folder == null && dropZone(e, a) === 'merge';
    a.classList.toggle('merge', merge);
    a.classList.toggle('drop', !merge);
  });
  a.addEventListener('dragleave', () => a.classList.remove('drop', 'merge'));
  a.addEventListener('drop', (e) => {
    e.preventDefault();
    a.classList.remove('drop', 'merge');
    let src;
    try {
      src = JSON.parse(e.dataTransfer.getData(DRAG));
    } catch {
      return;
    }
    if (!src || src.id === s.id) return;
    const index = list.findIndex((x) => x.id === s.id);
    const merge = folder == null && dropZone(e, a) === 'merge' && !src.folder;
    if (merge && s.folder) {
      mizu.request('shortcut_move', { id: src.id, index: 999, into: s.id }).then(load);
    } else if (merge) {
      mizu.request('shortcut_folder', { id: src.id, onto: s.id, title: i18n.tr('Folder') }).then(load);
    } else if (index >= 0) {
      mizu.request('shortcut_move', { id: src.id, index, into: folder ?? 0 }).then(load);
    }
  });
}

function tiles() {
  const box = h('div', { class: 'nt-tiles' });
  const list = data.top || [];
  for (const s of list) box.append(s.folder ? folderTile(s, list) : linkTile(s, list, null));
  if (!info.private) {
    const add = h('button', { class: 'nt-tile add', title: 'Add shortcut' }, h('div', { class: 'fav' }, icon('plus', { size: 20 })), h('span', { text: 'Add' }));
    add.addEventListener('click', () => tileDialog(null));
    box.append(add);
  }
  return box;
}

// An open folder: its name (editable) and its tiles. Redrawn after every change.
let folderView = null;
function openFolder(id) {
  const name = h('input', { type: 'text', class: 'nt-folder-name', maxlength: '40', 'aria-label': 'Folder name' });
  const grid = h('div', { class: 'nt-tiles nt-folder-grid' });
  const hint = h('div', { class: 'nt-folder-hint', text: 'Drag a tile onto another on the new tab page to make a folder. Drag here to reorder.' });
  const done = h('button', { class: 'btn primary', text: 'Done', onclick: () => dlg.close() });
  const dlg = h('dialog', { class: 'nt-folder' }, name, grid, h('div', { class: 'row' }, hint, done));
  let saveT = 0;
  name.addEventListener('input', () => {
    clearTimeout(saveT);
    saveT = setTimeout(() => mizu.request('shortcut_update', { id, title: name.value.trim() || i18n.tr('Folder'), url: '' }).then(load), 400);
  });
  name.addEventListener('keydown', (e) => e.key === 'Enter' && dlg.close());
  const refresh = () => {
    const f = (data.top || []).find((x) => x.id === id && x.folder);
    if (!f) return dlg.close();
    if (document.activeElement !== name) name.value = f.title || '';
    const items = f.items || [];
    grid.className = `nt-tiles nt-folder-grid size-${data.tile_size || 'medium'}`;
    grid.replaceChildren(...items.map((x) => linkTile(x, items, id)));
    const add = h('button', { class: 'nt-tile add', title: 'Add shortcut' }, h('div', { class: 'fav' }, icon('plus', { size: 20 })), h('span', { text: 'Add' }));
    add.addEventListener('click', () => tileDialog(null, id));
    grid.append(add);
  };
  folderView = { refresh };
  document.body.append(dlg);
  refresh();
  dlg.showModal();
  dlg.addEventListener('close', () => {
    clearTimeout(saveT);
    folderView = null;
    dlg.remove();
  });
}

function render() {
  const isPrivate = !!info.private || !!data.private || document.documentElement.hasAttribute('data-private');
  const clock = h('div', { class: 'nt-clock' });
  const date = h('div', { class: 'nt-greet' });
  const tick = () => {
    const now = new Date();
    clock.textContent = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    const day = now.toLocaleDateString(i18n.lang === 'ru' ? 'ru-RU' : 'en-US', { weekday: 'long', day: 'numeric', month: 'long' });
    const hello = i18n.tr(greeting());
    date.textContent = `${data.name ? `${hello}, ${data.name}` : hello} · ${day}`;
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
    if (data.clock !== false) content.append(h('div', { class: 'nt-hello' }, clock, date));
    // Widgets under the clock: what plays now.
    if (data.now_playing !== false) content.append(nowPlaying());
    content.append(searchForm());
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

  if (!isPrivate && data.engines && data.shortcuts !== false) {
    const t = tiles();
    t.classList.add(`size-${data.tile_size || 'medium'}`);
    content.append(t);
  }
  if (!isPrivate && data.engines && (data.recent || data.notes)) {
    content.append(h('div', { class: 'nt-widgets' }, data.recent ? recentWidget() : null, data.notes ? notesWidget() : null));
  }
  if (!isPrivate && data.engines && data.shield_stats !== false) {
    const stats = h('a', { class: 'nt-stats', href: '#' }, icon('shield-check', { size: 16 }), h('span', { text: `${Number(data.blocked_lifetime || 0)} ads and trackers blocked by Mizu Shield` }));
    stats.addEventListener('click', (e) => {
      e.preventDefault();
      mizu.send('open_url', { url: 'mizu://shield/', disposition: 'current' });
    });
    content.append(stats);
  }
  const bg = isPrivate ? 'wave' : data.background || 'wave';
  applyBackground(bg);
  main.replaceChildren(...[content, bg === 'wave' || isPrivate ? h('div', { class: 'nt-wave', 'aria-hidden': 'true' }) : null, !isPrivate && data.engines ? customizeButton() : null].filter(Boolean));
}

// ── Widgets ─────────────────────────────────────────────────
// "Now playing": the latest media tab with play/pause and next; hidden while nothing plays.
const npBox = h('div', { class: 'nt-np', hidden: true });
function drawNowPlaying(items) {
  const m = (items || [])[0];
  npBox.hidden = !m;
  if (!m) return npBox.replaceChildren();
  const send = (action) => mizu.request('media_control', { tab: m.tab, action });
  const btn = (ic, label, action, main) => {
    const b = h('button', { class: `nt-np-btn${main ? ' main' : ''}`, title: label }, icon(ic, { size: main ? 18 : 15 }));
    b.addEventListener('click', () => send(action));
    return b;
  };
  const art = m.artwork ? h('div', { class: 'nt-np-art', style: { backgroundImage: `url("${m.artwork.replace(/"/g, '%22')}")` } }) : h('div', { class: 'nt-np-art none' }, icon(m.video ? 'media' : 'music', { size: 20 }));
  const meta = h('button', { class: 'nt-np-meta', title: 'Go to tab' }, h('div', { class: 't', translate: 'no', text: m.title || m.page_title || m.site }), h('div', { class: 'd', translate: 'no', text: [m.artist, m.site].filter(Boolean).join(' · ') }));
  meta.addEventListener('click', () => send('focus'));
  npBox.classList.toggle('playing', !!m.playing);
  npBox.replaceChildren(art, meta, h('div', { class: 'nt-np-ctl' }, m.prev ? btn('skip-back', 'Previous track', 'prev') : null, btn(m.playing ? 'pause' : 'play', m.playing ? 'Pause' : 'Play', 'toggle', true), btn('skip-forward', 'Next track', 'next')));
}
function nowPlaying() {
  drawNowPlaying(data.media);
  return npBox;
}
mizu.on('media', (m) => {
  data.media = m.items;
  if (data.now_playing !== false) drawNowPlaying(m.items);
});

function recentWidget() {
  const list = h('div', { class: 'nt-recent' });
  const items = data.closed || [];
  if (!items.length) list.append(h('div', { class: 'nt-w-empty', text: 'No closed tabs yet.' }));
  for (const c of items) {
    const a = h('a', { href: c.url, class: 'nt-recent-row', title: c.url }, letterAvatar(c.url, c.title), h('span', { translate: 'no', text: c.title || hostOf(c.url) || c.url }));
    a.addEventListener('click', (e) => {
      e.preventDefault();
      mizu.send('open_url', { url: c.url, disposition: e.ctrlKey ? 'background' : 'current' });
    });
    list.append(a);
  }
  return h('section', { class: 'nt-widget' }, h('h4', null, icon('history', { size: 15 }), h('span', { text: 'Recently closed' })), list);
}

// Notes stay in this page's local storage, on this PC only.
const NOTES_KEY = 'mizu.newtab.notes';
function notesWidget() {
  const area = h('textarea', { class: 'nt-notes', placeholder: 'Write something down…', spellcheck: 'true' });
  area.value = store.get(NOTES_KEY) || '';
  let t = 0;
  area.addEventListener('input', () => {
    clearTimeout(t);
    t = setTimeout(() => store.set(NOTES_KEY, area.value.slice(0, 20000)), 300);
  });
  return h('section', { class: 'nt-widget' }, h('h4', null, icon('edit', { size: 15 }), h('span', { text: 'Notes' })), area);
}

// Customize: background (built-in or your picture), what the page shows, a link to the
// theme. Everything is saved right away.
const BACKGROUNDS = [
  ['wave', 'Wave'],
  ['accent', 'Accent'],
  ['dawn', 'Dawn'],
  ['dusk', 'Dusk'],
  ['aurora', 'Aurora'],
  ['sea', 'Sea'],
  ['forest', 'Forest'],
  ['sakura', 'Cherry blossom'],
  ['ember', 'Ember'],
  ['nebula', 'Nebula'],
  ['glacier', 'Glacier'],
  ['graphite', 'Graphite'],
  ['none', 'None'],
];
// Your picture stays in this page's local storage (resized), never in settings files.
const PIC_KEY = 'mizu.newtab.picture';
const DIM_KEY = 'mizu.newtab.dim';
const store = {
  get(k) {
    try {
      return localStorage.getItem(k);
    } catch {
      return null;
    }
  },
  set(k, v) {
    try {
      if (v === null) localStorage.removeItem(k);
      else localStorage.setItem(k, v);
      return true;
    } catch {
      return false;
    }
  },
};

function applyBackground(bg) {
  const pic = bg === 'image' ? store.get(PIC_KEY) : null;
  document.body.dataset.ntbg = bg === 'image' && !pic ? 'wave' : bg;
  if (pic) {
    const dim = Number(store.get(DIM_KEY) ?? 35) / 100;
    document.body.style.backgroundImage = `linear-gradient(rgb(0 0 0 / ${dim}), rgb(0 0 0 / ${dim})), url("${pic}")`;
  } else {
    document.body.style.backgroundImage = '';
  }
}

// Shrinks a picked image to at most 2560 px and stores it as JPEG.
async function pictureFromFile(file) {
  const bitmap = await createImageBitmap(file);
  const scale = Math.min(1, 2560 / Math.max(bitmap.width, bitmap.height));
  const canvas = document.createElement('canvas');
  canvas.width = Math.round(bitmap.width * scale);
  canvas.height = Math.round(bitmap.height * scale);
  canvas.getContext('2d').drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  for (const q of [0.85, 0.7, 0.55]) {
    const url = canvas.toDataURL('image/jpeg', q);
    if (url.length < 4_000_000 && store.set(PIC_KEY, url)) return true;
  }
  return false;
}

function customizeButton() {
  const btn = h('button', { class: 'nt-customize', title: 'Customize' }, icon('edit', { size: 15 }), h('span', { text: 'Customize' }));
  btn.addEventListener('click', openCustomize);
  return btn;
}

function openCustomize() {
  const prefs = {
    background: data.background || 'wave',
    clock: data.clock !== false,
    shield_stats: data.shield_stats !== false,
    shortcuts: data.shortcuts !== false,
    now_playing: data.now_playing !== false,
    recent: !!data.recent,
    notes: !!data.notes,
    tile_size: data.tile_size || 'medium',
    name: data.name || '',
  };
  const save = () => {
    Object.assign(data, prefs);
    mizu.request('newtab_prefs', prefs).then(render);
  };
  const choose = (id) => {
    prefs.background = id;
    for (const x of swatches.querySelectorAll('.nt-swatch')) x.classList.toggle('on', x.dataset.bg === id);
    dimRow.hidden = id !== 'image';
    save();
  };
  const swatch = (id, label, extra) => {
    const b = h('button', { class: `nt-swatch bg-${id}${prefs.background === id ? ' on' : ''}`, title: label, dataset: { bg: id } }, h('span', { class: 'nt-sw-check' }, icon('check', { size: 12 })), extra || null, h('span', { class: 'nt-sw-label', text: label }));
    b.addEventListener('click', () => choose(id));
    return b;
  };

  // "Your picture": a file picker; the tile shows the picture once there is one.
  const file = h('input', { type: 'file', accept: 'image/*', hidden: true });
  const picTile = swatch('image', 'Your picture', h('span', { class: 'nt-sw-plus' }, icon('upload', { size: 16 })));
  const showPic = () => {
    const pic = store.get(PIC_KEY);
    picTile.style.backgroundImage = pic ? `url("${pic}")` : '';
    picTile.classList.toggle('has-pic', !!pic);
  };
  showPic();
  picTile.addEventListener('click', (e) => {
    if (!store.get(PIC_KEY) || e.target.closest('.nt-sw-plus')) {
      e.stopImmediatePropagation();
      file.click();
    }
  }, true);
  file.addEventListener('change', async () => {
    const f = file.files?.[0];
    file.value = '';
    if (!f) return;
    if (await pictureFromFile(f).catch(() => false)) {
      showPic();
      choose('image');
    } else {
      showToast('This picture could not be used');
    }
  });

  const swatches = h('div', { class: 'nt-swatches' }, ...BACKGROUNDS.map(([id, label]) => swatch(id, label)), picTile, file);

  const dim = h('input', { type: 'range', min: '0', max: '80', value: store.get(DIM_KEY) ?? '35' });
  dim.addEventListener('input', () => {
    store.set(DIM_KEY, dim.value);
    applyBackground('image');
  });
  const removePic = h('button', { class: 'btn ghost', text: 'Remove picture' });
  removePic.addEventListener('click', () => {
    store.set(PIC_KEY, null);
    showPic();
    choose('wave');
  });
  const dimRow = h('div', { class: 'nt-dim' }, h('span', { text: 'Darken picture' }), dim, removePic);
  dimRow.hidden = prefs.background !== 'image';

  const check = (label, key) => h('div', { class: 'nt-check' }, h('span', { text: label }), switchInput(prefs[key], (v) => {
    prefs[key] = v;
    save();
  }, { label }));

  const sizeSeg = h('div', { class: 'nt-seg' }, ...[['small', 'Small'], ['medium', 'Medium'], ['large', 'Large']].map(([v, label]) => {
    const b = h('button', { class: prefs.tile_size === v ? 'on' : '', text: label });
    b.addEventListener('click', () => {
      prefs.tile_size = v;
      for (const x of b.parentElement.children) x.classList.toggle('on', x === b);
      save();
    });
    return b;
  }));
  const nameInput = h('input', { type: 'text', class: 'nt-name', value: prefs.name, maxlength: '40', placeholder: 'Optional' });
  let nameT = 0;
  nameInput.addEventListener('input', () => {
    clearTimeout(nameT);
    nameT = setTimeout(() => {
      prefs.name = nameInput.value.trim();
      save();
    }, 400);
  });
  const theme = h('button', { class: 'btn ghost nt-theme-link' }, icon('sun', { size: 15 }), h('span', { text: 'Colours and theme' }));
  theme.addEventListener('click', () => mizu.send('open_url', { url: 'mizu://settings/#appearance', disposition: 'current' }));
  const done = h('button', { class: 'btn primary', text: 'Done', autofocus: true });
  const dlg = h('dialog', { class: 'nt-custom' },
    h('h3', { text: 'Customize new tab' }),
    h('div', { class: 'nt-sub', text: 'Background' }), swatches, dimRow,
    h('div', { class: 'nt-sub', text: 'Show on the page' }),
    h('div', { class: 'nt-checks' }, check('Clock and greeting', 'clock'), check('Shortcut tiles', 'shortcuts'), check('Mizu Shield counter', 'shield_stats')),
    h('div', { class: 'nt-sub', text: 'Widgets' }),
    h('div', { class: 'nt-checks' }, check('Now playing', 'now_playing'), check('Recently closed', 'recent'), check('Notes', 'notes')),
    h('div', { class: 'nt-sub', text: 'Layout' }),
    h('div', { class: 'nt-checks' },
      h('div', { class: 'nt-check' }, h('span', { text: 'Tile size' }), sizeSeg),
      h('div', { class: 'nt-check' }, h('span', { text: 'Your name in the greeting' }), nameInput)),
    h('div', { class: 'row' }, theme, done));
  done.addEventListener('click', () => dlg.close());
  dlg.addEventListener('close', () => dlg.remove());
  document.body.append(dlg);
  dlg.showModal();
}

function load() {
  return mizu
    .request('newtab_data')
    .then((d) => {
      data = d || {};
      if (!engineId) engineId = data.engine_id;
      render();
      if (folderView) folderView.refresh();
      if (location.hash === '#customize') openCustomize();
      const m = /^#folder=(\d+)$/.exec(location.hash);
      if (m && !folderView) openFolder(Number(m[1]));
    })
    .catch(() => render());
}

// Draw immediately; fill in tiles when the browser answers.
render();
bootPage((i) => {
  info = i || {};
  load();
});
