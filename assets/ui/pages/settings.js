// Settings: ten categories bound to the browser's settings object.
'use strict';

const main = document.getElementById('main');
let payload = null;
let s = null; // working copy of settings
let current = (location.hash || '#general').slice(1);
let saveTimer = 0;

const CATEGORIES = [
  ['general', 'General', 'sliders'],
  ['appearance', 'Appearance', 'sun'],
  ['search', 'Search', 'search'],
  ['privacy', 'Privacy & Shield', 'shield'],
  ['security', 'Security', 'lock'],
  ['downloads', 'Downloads', 'download'],
  ['tabs', 'Tabs', 'tabs'],
  ['performance', 'Performance', 'gauge'],
  ['media', 'Media', 'media'],
  ['shortcuts', 'Shortcuts', 'keyboard'],
  ['advanced', 'Advanced', 'code'],
];

function save(immediate) {
  clearTimeout(saveTimer);
  saveTimer = 0;
  const run = () =>
    mizu
      .request('set_settings', { settings: s })
      .then((p) => {
        // Keep `s` as the single working copy: controls on screen hold references into it.
        // Only adopt the browser's copy if it corrected something (sanitised values).
        payload = p;
        if (stripDerived(p.settings) !== stripDerived(s)) {
          s = structuredClone(p.settings);
          renderContent();
        }
        if (p.restart_needed) showToast('Saved — restart Mizu to apply this change');
        else showToast('Saved');
        if (current === 'privacy') renderContent();
      })
      .catch((e) => showToast(e.message));
  if (immediate) run();
  else
    saveTimer = setTimeout(() => {
      saveTimer = 0;
      run();
    }, 350);
}

function stripDerived(obj) {
  const { _dark, _engine, ...rest } = obj || {};
  return JSON.stringify(rest);
}

// ── Control builders ─────────────────────────────────────────
function setting(title, desc, control) {
  return h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: title }), desc ? h('div', { class: 'd', text: desc }) : null), control);
}
function toggle(title, desc, get, set, opts) {
  return setting(title, desc, switchInput(get(), (v) => {
    set(v);
    save();
  }, { label: title, ...opts }));
}
function seg(options, value, onPick) {
  const el = h('div', { class: 'seg', role: 'radiogroup' });
  for (const [v, label] of options) {
    const b = h('button', { class: v === value ? 'on' : '', role: 'radio', 'aria-checked': String(v === value), text: label });
    b.addEventListener('click', () => {
      for (const c of el.children) c.className = '';
      b.className = 'on';
      onPick(v);
    });
    el.append(b);
  }
  return el;
}
function select(options, value, onPick) {
  const el = h('select');
  for (const [v, label] of options) {
    const o = h('option', { value: String(v), text: label });
    if (String(v) === String(value)) o.selected = true;
    el.append(o);
  }
  el.addEventListener('change', () => onPick(el.value));
  return el;
}
function card(title, note, ...rows) {
  return h('section', { class: 'card' }, title ? h('h2', { text: title }) : null, note ? h('p', { class: 'note', text: note }) : null, ...rows);
}

// ── Categories ───────────────────────────────────────────────
function general() {
  const g = s.general;
  const home = h('input', { type: 'url', value: g.home_page, style: { width: '320px' } });
  home.addEventListener('change', () => {
    g.home_page = home.value.trim();
    save();
  });
  const lang = select([['auto', 'Automatic'], ['en', 'English'], ['ru', 'Русский']], g.language || 'auto', (v) => {
    g.language = v;
    save(true);
  });
  return [
    card('Language', null, setting('Language', 'Interface language. "Automatic" follows Windows.', lang)),
    card('On startup', null,
      setting('When Mizu starts', null, seg([['restore_session', 'Continue where I left off'], ['new_tab', 'New tab'], ['home_page', 'Home page']], g.startup, (v) => {
        g.startup = v;
        save();
      })),
      setting('Home page', 'Used by the Home button and on startup.', home),
      toggle('Offer to restore tabs after a crash', 'If Mizu didn\'t close normally, your tabs come back.', () => g.restore_after_crash, (v) => (g.restore_after_crash = v)),
      toggle('Confirm before closing many tabs', null, () => g.confirm_close_multiple_tabs, (v) => (g.confirm_close_multiple_tabs = v)),
      toggle('Open windows maximized', 'Mizu starts full-size instead of in a smaller window.', () => g.start_maximized, (v) => (g.start_maximized = v)),
    ),
    updatesCard(g),
  ];
}

const PALETTES = [
  ['mizu', 'Mizu', '#2f7de1', '#e9eff5'],
  ['washi', 'Washi', '#b07a3c', '#f1ede6'],
  ['matcha', 'Matcha', '#4f8a4b', '#e9efe6'],
  ['sumi', 'Sumi', '#333333', '#eeeeee'],
  ['sakura', 'Sakura', '#d0607b', '#f6ecee'],
];
const ACCENTS = ['#2f7de1', '#0f9fb5', '#2e9d6a', '#7a5af5', '#d0607b', '#e0672b', '#c99a12', '#4b5563'];

function appearance() {
  const a = s.appearance;
  const palettes = h('div', { class: 'swatches' });
  for (const [id, label, accent, bg] of PALETTES) {
    const b = h('button', { class: `pal${a.palette === id ? ' on' : ''}`, title: label }, h('span', { class: 'chip', style: { background: `linear-gradient(135deg, ${bg} 50%, ${accent} 50%)` } }), h('span', { text: label }));
    b.addEventListener('click', () => {
      a.palette = id;
      a.accent = accent === '#333333' ? a.accent : accent;
      save(true);
      renderContent();
    });
    palettes.append(b);
  }
  const accents = h('div', { class: 'accents' });
  for (const c of ACCENTS) {
    const b = h('button', { class: `acc${a.accent.toLowerCase() === c ? ' on' : ''}`, style: { background: c }, title: c });
    b.addEventListener('click', () => {
      a.accent = c;
      save(true);
      renderContent();
    });
    accents.append(b);
  }
  const custom = h('input', { type: 'color', value: a.accent, title: 'Custom colour' });
  custom.addEventListener('change', () => {
    a.accent = custom.value;
    save(true);
  });
  accents.append(custom);
  const zoom = select([[67, '67%'], [75, '75%'], [80, '80%'], [90, '90%'], [100, '100%'], [110, '110%'], [125, '125%'], [150, '150%']], a.zoom_percent, (v) => {
    a.zoom_percent = Number(v);
    save();
  });
  return [
    card('Theme', null,
      setting('Mode', null, seg([['light', 'Light'], ['dark', 'Dark'], ['system', 'System']], a.theme, (v) => {
        a.theme = v;
        save(true);
      })),
      h('div', { class: 'setting col' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'Palette' })), palettes),
      h('div', { class: 'setting col' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'Accent colour' })), accents),
    ),
    card('Interface', null,
      setting('Style', 'Soft uses rounded, floating controls; Crisp is flatter.', seg([['soft', 'Soft'], ['crisp', 'Crisp']], a.style, (v) => {
        a.style = v;
        save(true);
      })),
      setting('Density', null, seg([['compact', 'Compact'], ['comfortable', 'Comfortable'], ['spacious', 'Spacious']], a.density, (v) => {
        a.density = v;
        save(true);
      })),
      toggle('Animations', 'Smooth transitions in the interface. Windows "reduce motion" is always respected.', () => a.animations, (v) => (a.animations = v)),
      toggle('Translucent window (Mica)', 'Lets the desktop colour show through the tab strip on Windows 11.', () => a.translucency, (v) => (a.translucency = v)),
      setting('Tab layout', 'Vertical tabs live in a sidebar on the left; collapse it to icons with the arrow at its bottom.', seg([['top', 'Top'], ['side', 'Side']], a.tab_layout || 'top', (v) => {
        a.tab_layout = v;
        save(true);
      })),
      toggle('Show bookmarks bar', 'Ctrl+Shift+B', () => a.show_bookmarks_bar, (v) => (a.show_bookmarks_bar = v)),
      toggle('Show Home button', null, () => a.show_home_button, (v) => (a.show_home_button = v)),
      setting('Default page zoom', null, zoom),
    ),
  ];
}

function search() {
  const sr = s.search;
  const engines = payload.engines || [];
  const list = h('div');
  for (const e of sr.custom_engines) {
    const del = h('button', { class: 'icon-btn', title: 'Remove' }, icon('trash'));
    del.addEventListener('click', () => {
      sr.custom_engines = sr.custom_engines.filter((x) => x.id !== e.id);
      if (sr.engine === e.id) sr.engine = 'duckduckgo';
      save(true);
      renderContent();
    });
    list.append(h('div', { class: 'list-row' }, letterAvatar(e.template, e.name), h('div', { class: 'main' }, h('div', { class: 't', text: `${e.name}${e.keyword ? `  ·  ${e.keyword}` : ''}` }), h('div', { class: 'u', text: e.template })), h('div', { class: 'acts' }, del)));
  }
  const add = h('button', { class: 'btn', text: 'Add search engine' });
  add.addEventListener('click', () => {
    const name = h('input', { type: 'text', placeholder: 'Name' });
    const kw = h('input', { type: 'text', placeholder: '@keyword (optional)' });
    const url = h('input', { type: 'url', placeholder: 'https://example.com/search?q={searchTerms}' });
    const dlg = h('dialog', null, h('h3', { text: 'Add search engine' }),
      h('div', { class: 'fields' }, h('label', null, 'Name', name), h('label', null, 'Keyword', kw), h('label', null, 'URL with {searchTerms}', url)),
      h('div', { class: 'actions' }, h('button', { class: 'btn', text: 'Cancel', onclick: () => dlg.close() }), h('button', { class: 'btn primary', text: 'Add', onclick: () => {
        const t = url.value.trim();
        if (!name.value.trim() || !/^https?:\/\//.test(t) || !t.includes('{searchTerms}')) {
          showToast('Enter a name and an http(s) URL containing {searchTerms}');
          return;
        }
        sr.custom_engines.push({ id: `custom-${Date.now().toString(36)}`, name: name.value.trim(), template: t, keyword: kw.value.trim() });
        dlg.close();
        save(true);
        setTimeout(renderContent, 300);
      } })));
    document.body.append(dlg);
    dlg.showModal();
    dlg.addEventListener('close', () => dlg.remove());
  });
  return [
    card('Address bar', null,
      setting('Search engine', 'Used when you type something that isn\'t an address. Type a keyword like @w to search a specific engine.', select(engines.map((e) => [e.id, e.keyword ? `${e.name}  (${e.keyword})` : e.name]), sr.engine, (v) => {
        sr.engine = v;
        save();
      })),
      toggle('Search suggestions', 'Shows popular queries from your search engine while you type. What you type is sent to that engine; never in private windows.', () => sr.remote_suggestions, (v) => (sr.remote_suggestions = v)),
      toggle('Suggest from history and bookmarks', 'Suggestions are computed on this device; nothing you type is sent anywhere until you press Enter.', () => sr.local_suggestions, (v) => (sr.local_suggestions = v)),
    ),
    card('Custom search engines', null, list, h('div', { class: 'setting' }, h('div', { class: 'txt' }), add)),
  ];
}

function filterLists() {
  const p = s.privacy;
  const box = h('div');
  for (const l of payload.lists || []) {
    const sub = p.filter_lists.find((x) => x.id === l.id);
    if (!sub) continue;
    const status = l.last_error
      ? h('span', { class: 'pill bad', text: 'Update failed' })
      : l.downloaded
        ? h('span', { class: 'pill ok', text: l.last_updated ? `Updated ${relativeTime(l.last_updated)}` : 'Built in' })
        : h('span', { class: 'pill', text: sub.enabled ? 'Waiting for download' : 'Not downloaded' });
    const acts = h('div', { class: 'acts', style: { opacity: 1 } });
    if (!l.builtin) {
      const del = h('button', { class: 'icon-btn', title: 'Remove list' }, icon('trash'));
      del.addEventListener('click', () => mizu.request('filter_list_remove', { id: l.id }).then((pl) => {
        payload = pl;
        s = structuredClone(pl.settings);
        renderContent();
      }));
      acts.append(del);
    }
    const sw = switchInput(sub.enabled, (v) => {
      sub.enabled = v;
      save();
    }, { label: l.name });
    box.append(h('div', { class: 'list-row' }, h('div', { class: 'main' }, h('div', { class: 't', text: l.name }), h('div', { class: 'u', title: l.last_error || l.url, text: l.url || 'Shipped with Mizu' })), status, l.bytes ? h('span', { class: 'when', text: formatBytes(l.bytes) }) : null, acts, sw));
  }
  return box;
}

function privacy() {
  const p = s.privacy;
  const update = h('button', { class: 'btn', text: payload.filters_updating ? 'Updating…' : 'Update lists now', disabled: payload.filters_updating });
  update.addEventListener('click', () => {
    mizu.send('filters_update', { force: true });
    update.textContent = 'Updating…';
    update.disabled = true;
  });
  const addList = h('button', { class: 'btn', text: 'Add list…' });
  addList.addEventListener('click', () => {
    const name = h('input', { type: 'text', placeholder: 'My list' });
    const url = h('input', { type: 'url', placeholder: 'https://…/list.txt' });
    const dlg = h('dialog', null, h('h3', { text: 'Subscribe to a filter list' }),
      h('p', { class: 'muted', text: 'Any list in uBlock Origin / Adblock Plus syntax, or a hosts file.' }),
      h('div', { class: 'fields' }, h('label', null, 'Name', name), h('label', null, 'HTTPS address', url)),
      h('div', { class: 'actions' }, h('button', { class: 'btn', text: 'Cancel', onclick: () => dlg.close() }), h('button', { class: 'btn primary', text: 'Subscribe', onclick: () => {
        mizu.request('filter_list_add', { name: name.value, url: url.value.trim() }).then((pl) => {
          payload = pl;
          s = structuredClone(pl.settings);
          dlg.close();
          renderContent();
        }).catch((e) => showToast(e.message));
      } })));
    document.body.append(dlg);
    dlg.showModal();
    dlg.addEventListener('close', () => dlg.remove());
  });

  const rules = h('textarea', { rows: 6, placeholder: '! One rule per line, uBlock Origin syntax\n||ads.example.com^\nexample.com##.banner', spellcheck: 'false' });
  rules.value = p.custom_rules;
  const saveRules = h('button', { class: 'btn', text: 'Apply rules' });
  saveRules.addEventListener('click', () => {
    p.custom_rules = rules.value;
    save(true);
  });

  const allow = h('div');
  if (!p.allowlist.length) allow.append(h('p', { class: 'note', style: { padding: '0 20px 12px', color: 'var(--ink-3)' }, text: 'No exceptions. Turn Shield off for a site from the shield button in the address bar.' }));
  for (const host of p.allowlist) {
    const del = h('button', { class: 'icon-btn', title: 'Remove exception' }, icon('trash'));
    del.addEventListener('click', () => {
      p.allowlist = p.allowlist.filter((x) => x !== host);
      save(true);
      renderContent();
    });
    allow.append(h('div', { class: 'list-row' }, letterAvatar(`https://${host}`, host), h('div', { class: 'main' }, h('div', { class: 't', text: host })), h('div', { class: 'acts', style: { opacity: 1 } }, del)));
  }

  return [
    card('Mizu Shield', 'Blocks ads, trackers and known malicious content with filter lists in uBlock Origin syntax, on top of the engine\'s own tracking prevention.',
      toggle('Block ads and trackers', 'Network filtering, element hiding and scriptlets, like uBlock Origin.', () => p.content_blocking, (v) => (p.content_blocking = v)),
      setting('Tracking prevention', 'Engine-level protection against cross-site tracking storage. Strict may break some sign-ins.', seg([['off', 'Off'], ['basic', 'Basic'], ['balanced', 'Balanced'], ['strict', 'Strict']], p.tracking_prevention, (v) => {
        p.tracking_prevention = v;
        save();
      })),
      toggle('Send Global Privacy Control', 'Tells websites not to sell or share your data (Sec-GPC header). Legally binding in some regions.', () => p.global_privacy_control, (v) => (p.global_privacy_control = v)),
    ),
    card('Filter lists', 'Lists are downloaded over HTTPS directly from their maintainers and refreshed every few days.', filterLists(), h('div', { class: 'setting' }, h('div', { class: 'txt' }), addList, update)),
    card('My rules', null, h('div', { class: 'setting col' }, rules, h('div', null, saveRules))),
    card('Sites where Shield is off', null, allow),
    clearDataCard(),
    card('Clear when Mizu closes', null,
      toggle('History', null, () => p.clear_on_exit.history, (v) => (p.clear_on_exit.history = v)),
      toggle('Cookies', 'You will be signed out of websites.', () => p.clear_on_exit.cookies, (v) => (p.clear_on_exit.cookies = v)),
      toggle('Cached files', null, () => p.clear_on_exit.cache, (v) => (p.clear_on_exit.cache = v)),
      toggle('Site storage', 'Local storage, IndexedDB, service workers.', () => p.clear_on_exit.site_storage, (v) => (p.clear_on_exit.site_storage = v)),
      toggle('Download history', 'Files stay on disk.', () => p.clear_on_exit.downloads, (v) => (p.clear_on_exit.downloads = v)),
    ),
    permissionsCard(),
    card('Passwords', 'Mizu Passwords stores logins encrypted with your Windows account (DPAPI) on this device only.',
      toggle('Offer to save passwords', 'After you sign in, Mizu asks whether to remember the login. Fill it later with the key in the address bar.', () => p.password_manager, (v) => (p.password_manager = v)),
      setting('Saved passwords', 'View, edit, import from other browsers or export.', (() => {
        const b = h('button', { class: 'btn', text: 'Manage passwords' });
        b.addEventListener('click', () => mizu.send('open_url', { url: 'mizu://passwords/', disposition: 'new_tab' }));
        return b;
      })()),
      toggle('Engine form autofill', 'Lets the web engine remember addresses and other form entries.', () => p.engine_autofill, (v) => (p.engine_autofill = v)),
    ),
  ];
}

function clearDataCard() {
  const opts = { history: true, cookies: false, cache: true, site_storage: false, downloads: false, permissions: false, autofill: false, passwords: false };
  const labels = { history: 'Browsing history', cookies: 'Cookies', cache: 'Cached images and files', site_storage: 'Site storage (local storage, IndexedDB, service workers)', downloads: 'Download history', permissions: 'Site permissions', autofill: 'Autofill data', passwords: 'Saved passwords' };
  const grid = h('div', { class: 'checks' });
  for (const k of Object.keys(opts)) {
    const cb = h('input', { type: 'checkbox', checked: opts[k] });
    cb.addEventListener('change', () => (opts[k] = cb.checked));
    grid.append(h('label', null, cb, labels[k]));
  }
  let range = 'hour';
  const btn = h('button', { class: 'btn danger', text: 'Clear data' });
  btn.addEventListener('click', () => {
    btn.disabled = true;
    mizu.request('clear_data', { range, ...opts }).then(() => showToast('Browsing data cleared')).catch((e) => showToast(e.message)).finally(() => (btn.disabled = false));
  });
  return card('Clear browsing data', null, h('div', { class: 'setting col' }, grid), setting('Time range', null, h('div', { style: { display: 'flex', gap: '10px' } }, select([['hour', 'Last hour'], ['day', 'Last 24 hours'], ['week', 'Last 7 days'], ['month', 'Last 4 weeks'], ['all', 'All time']], range, (v) => (range = v)), btn)));
}

function permissionsCard() {
  const body = h('div', null, h('p', { class: 'note', style: { padding: '0 20px 12px', color: 'var(--ink-3)' }, text: 'Loading…' }));
  mizu.request('permissions_get').then((list) => {
    body.replaceChildren();
    if (!list.length) {
      body.append(h('p', { class: 'note', style: { padding: '0 20px 12px', color: 'var(--ink-3)' }, text: 'Sites ask before using your camera, microphone, location, notifications or clipboard. Remembered answers appear here.' }));
      return;
    }
    const table = h('table', { class: 'grid' }, h('tr', null, h('th', { text: 'Site' }), h('th', { text: 'Permission' }), h('th', { text: 'Setting' }), h('th')));
    for (const p of list) {
      const st = select([['allow', 'Allow'], ['block', 'Block']], p.state, (v) => mizu.request('permission_set', { origin: p.origin, kind: p.kind, state: v }).then(() => showToast('Saved')));
      const del = h('button', { class: 'icon-btn', title: 'Reset' }, icon('trash'));
      const row = h('tr', null, h('td', { text: p.origin }), h('td', { text: p.kind.replace(/_/g, ' ') }), h('td', null, st), h('td', null, del));
      del.addEventListener('click', () => mizu.request('permission_set', { origin: p.origin, kind: p.kind, state: 'reset' }).then(() => row.remove()));
      table.append(row);
    }
    body.append(table);
  });
  return card('Site permissions', null, body);
}

function security() {
  const sec = s.security;
  return [
    card('Protection', null,
      toggle('Microsoft Defender SmartScreen', 'Warns about phishing and malware sites and downloads. Addresses you visit are checked with Microsoft\'s service.', () => sec.smartscreen, (v) => (sec.smartscreen = v)),
      toggle('Block pop-ups', 'Pop-ups that open without a click are blocked; you will see a notice.', () => sec.block_unrequested_popups, (v) => (sec.block_unrequested_popups = v)),
    ),
    card('Password manager', null,
      setting('Mizu Passwords', 'Your saved logins, encrypted with your Windows account. View, edit, import or export them.', (() => {
        const b = h('button', { class: 'btn primary' }, icon('key'), h('span', { text: 'Open password manager' }));
        b.addEventListener('click', () => mizu.send('open_url', { url: 'mizu://passwords/', disposition: 'new_tab' }));
        return b;
      })()),
    ),
    card('Always on', 'These protections cannot be turned off.',
      setting('Process sandbox and site isolation', 'Every site runs in a separate, sandboxed process of the Chromium engine.', h('span', { class: 'pill ok', text: 'On' })),
      setting('Certificate verification', 'Invalid HTTPS certificates are always rejected; there is no "proceed anyway".', h('span', { class: 'pill ok', text: 'On' })),
      setting('Internal page isolation', 'Websites cannot open or script Mizu\'s own pages.', h('span', { class: 'pill ok', text: 'On' })),
    ),
  ];
}

function downloads() {
  const d = s.downloads;
  const path = h('span', { class: 'mono', text: d.directory || payload.download_dir_default });
  const change = h('button', { class: 'btn', text: 'Change…' });
  change.addEventListener('click', () => mizu.request('choose_download_folder').then((r) => {
    if (r && r.path) {
      d.directory = r.path;
      path.textContent = r.path;
      showToast('Download folder changed');
    }
  }));
  return [
    card('Downloads', null,
      setting('Save files to', null, h('div', { style: { display: 'flex', gap: '10px', alignItems: 'center', minWidth: 0 } }, path, change)),
      toggle('Ask where to save each file', null, () => d.ask_where_to_save, (v) => (d.ask_where_to_save = v)),
      toggle('Show downloads when a download starts', null, () => d.open_panel_on_start, (v) => (d.open_panel_on_start = v)),
    ),
  ];
}

function tabs() {
  const t = s.tabs;
  return [
    card('Tabs', null,
      setting('Open new tabs', null, seg([['after_current', 'Next to current'], ['end', 'At the end']], t.new_tab_position, (v) => {
        t.new_tab_position = v;
        save();
      })),
      toggle('Switch to tabs opened from links', null, () => t.switch_to_new_tabs_from_links, (v) => (t.switch_to_new_tabs_from_links = v)),
    ),
    card('Sleeping tabs', 'Background tabs you haven\'t used for a while are suspended to save memory and battery. Tabs playing audio never sleep.',
      toggle('Put inactive tabs to sleep', null, () => t.sleeping_tabs, (v) => (t.sleeping_tabs = v)),
      setting('After', null, select([[5, '5 minutes'], [15, '15 minutes'], [30, '30 minutes'], [60, '1 hour'], [120, '2 hours'], [360, '6 hours']], t.sleep_after_minutes, (v) => {
        t.sleep_after_minutes = Number(v);
        save();
      })),
    ),
  ];
}

function performance() {
  const p = s.performance;
  return [
    card('Performance', null,
      toggle('Efficiency mode', 'Asks the engine to keep memory low for tabs you are not looking at.', () => p.efficiency_mode, (v) => (p.efficiency_mode = v)),
      toggle('Hardware acceleration', 'Uses the GPU for rendering and video. Requires restart.', () => p.hardware_acceleration, (v) => (p.hardware_acceleration = v)),
    ),
  ];
}

function media() {
  const m = s.media;
  const diag = h('button', { class: 'btn', text: 'Open media diagnostics' });
  diag.addEventListener('click', () => mizu.send('open_url', { url: 'mizu://media/', disposition: 'new_tab' }));
  return [
    card('Autoplay', null,
      setting('Media autoplay', 'Requires restart.', seg([['default', 'Muted autoplay allowed'], ['require_interaction', 'Only after I interact']], m.autoplay, (v) => {
        m.autoplay = v;
        save();
      })),
    ),
    card('Picture in picture', null,
      toggle('Picture-in-picture button on videos', 'Hover a video to pop it out into a floating window that stays on top. Works even where a site hides its own button.', () => m.pip_button, (v) => (m.pip_button = v)),
      setting('Button position', null, seg([['top-right', 'Top right'], ['top-left', 'Top left']], m.pip_position || 'top-right', (v) => {
        m.pip_position = v;
        save();
      })),
      toggle('Compact button', 'Show only the icon, without the "PiP" label.', () => m.pip_compact, (v) => (m.pip_compact = v)),
      toggle('Video switching in the floating window', "Adds previous/next video and 10-second skip buttons. On YouTube they use the player's own playlist; elsewhere they switch between the videos on the page.", () => m.pip_controls, (v) => (m.pip_controls = v)),
      setting('Keyboard shortcut', 'Pops out the playing video on any page, even with the button turned off. Reload open pages to apply changes.', h('span', { class: 'kbd-combo' }, h('kbd', { text: 'Alt' }), h('span', { class: 'kbd-plus', text: '+' }), h('kbd', { text: 'Shift' }), h('span', { class: 'kbd-plus', text: '+' }), h('kbd', { text: 'P' }))),
    ),
    card('Codecs', 'See which video and audio formats this system can decode, and whether hardware acceleration is available.', h('div', { class: 'setting' }, h('div', { class: 'txt' }), diag)),
  ];
}

function advanced() {
  const a = s.advanced;
  const reset = h('button', { class: 'btn danger', text: 'Reset all settings' });
  reset.addEventListener('click', () => {
    if (!confirm('Reset all settings to their defaults? Your history, bookmarks and passwords are not affected.')) return;
    mizu.request('reset_settings').then((p) => {
      payload = p;
      s = structuredClone(p.settings);
      renderContent();
      showToast('Settings reset');
    });
  });
  const scripts = h('button', { class: 'btn', text: 'Manage scripts & extensions' });
  scripts.addEventListener('click', () => mizu.send('open_url', { url: 'mizu://scripts/', disposition: 'new_tab' }));
  return [
    card('Developers', null,
      toggle('Developer tools', 'F12 and "Inspect" in the context menu.', () => a.developer_tools, (v) => (a.developer_tools = v)),
      toggle('Extensions', 'Allow Chromium extensions (Manifest V3), e.g. uBlock Origin Lite. Requires restart.', () => a.extensions, (v) => (a.extensions = v)),
      toggle('Update userscripts automatically', 'Checks each script\'s update source once a day and installs newer versions.', () => a.userscript_auto_update, (v) => (a.userscript_auto_update = v)),
      setting('Userscripts and extensions', null, scripts),
    ),
    card('Engine', null, setting('Microsoft Edge WebView2', 'Updated automatically by Windows, independently of Mizu.', h('span', { class: 'mono', text: payload.engine_version || 'unknown' }))),
    card('Reset', null, setting('Restore default settings', null, reset)),
  ];
}

// ── Updates (shown in General) ───────────────────────────────
function updateText(u, installed) {
  switch (u && u.state) {
    case 'checking':
      return 'Checking for updates…';
    case 'up_to_date':
      return 'Mizu is up to date.';
    case 'ready':
      return `Mizu ${u.version} is downloaded and verified. Restart to install it.`;
    case 'available':
      return `Mizu ${u.version} is available. This copy is not installed with MizuSetup, so download it from the releases page.`;
    case 'failed':
      return `Could not check for updates: ${u.error}`;
    default:
      return installed ? 'Updates are signed by Onibi Lab and installed automatically.' : 'Portable copy: updates are announced here; install Mizu with MizuSetup for automatic updates.';
  }
}

function updatesCard(g) {
  const status = h('div', { class: 'd', text: '' });
  const version = h('div', { class: 't', text: 'Mizu' });
  const check = h('button', { class: 'btn' }, icon('reload'), h('span', { text: 'Check for updates' }));
  const install = h('button', { class: 'btn primary' }, icon('upload'), h('span', { text: 'Update' }));
  let installed = false;
  const show = (u) => {
    status.textContent = i18n.tr(updateText(u, installed));
    status.classList.toggle('warn', !!u && u.state === 'failed');
    check.disabled = !!u && u.state === 'checking';
    const ready = !!u && u.state === 'ready';
    const available = !!u && u.state === 'available';
    install.hidden = !ready && !available;
    install.lastChild.textContent = i18n.tr(available ? 'Download' : 'Update');
    install.onclick = () => {
      if (available) mizu.send('open_url', { url: 'https://github.com/senzukisu/mizu/releases/latest', disposition: 'new_tab' });
      else mizu.request('update_install').catch((e) => showToast(e.message));
    };
  };
  check.addEventListener('click', () => {
    show({ state: 'checking' });
    mizu.request('update_check').then(show).catch((e) => show({ state: 'failed', error: e.message }));
  });
  install.hidden = true;
  mizu.request('update_state').then((r) => {
    installed = r.installed;
    version.textContent = `Mizu ${r.version}`;
    show(r.update);
  }).catch(() => show(null));
  return card('Updates', null,
    h('div', { class: 'setting' }, h('div', { class: 'txt' }, version, status), h('div', { class: 'upd-btns' }, check, install)),
    toggle('Update Mizu automatically', 'Checks for new versions when Mizu starts and every few hours, then asks before installing.', () => g.auto_update, (v) => (g.auto_update = v)),
  );
}

// ── Keyboard shortcuts ───────────────────────────────────────
// Shortcuts the engine or the page handles; shown for reference, not editable.
const FIXED_SHORTCUTS = [
  ['Go to tab 1–9', ['Ctrl+1…9']],
  ['Find on page', ['Ctrl+F']],
  ['Print', ['Ctrl+P']],
  ['Zoom in / out', ['Ctrl+Plus', 'Ctrl+Minus']],
  ['Reset zoom', ['Ctrl+0']],
  ['Picture in picture', ['Alt+Shift+P']],
  ['Developer tools', ['F12']],
];

// KeyboardEvent.code → the key names the browser understands.
function keyName(e) {
  const c = e.code;
  if (/^Key[A-Z]$/.test(c)) return c.slice(3);
  if (/^Digit[0-9]$/.test(c)) return c.slice(5);
  if (/^Numpad[0-9]$/.test(c)) return c.slice(6);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(c)) return c;
  const map = {
    ArrowLeft: 'Left', ArrowRight: 'Right', ArrowUp: 'Up', ArrowDown: 'Down', PageUp: 'PgUp', PageDown: 'PgDn',
    Home: 'Home', End: 'End', Insert: 'Insert', Delete: 'Delete', Tab: 'Tab', Space: 'Space', Enter: 'Enter',
    Backspace: 'Backspace', Equal: 'Plus', NumpadAdd: 'Plus', Minus: 'Minus', NumpadSubtract: 'Minus',
    Comma: 'Comma', Period: 'Period', Slash: 'Slash',
  };
  return map[c] || null;
}

function comboFromEvent(e) {
  const k = keyName(e);
  if (!k) return null;
  const parts = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  parts.push(k);
  return parts.join('+');
}

const sameCombo = (a, b) => a.toLowerCase() === b.toLowerCase();

function keys(combo) {
  const parts = combo.split('+');
  const out = [];
  parts.forEach((k, i) => {
    if (i) out.push(h('span', { class: 'kbd-plus', text: '+' }));
    out.push(h('kbd', { translate: 'no', text: k === 'Plus' ? '+' : k === 'Minus' ? '−' : k }));
  });
  return h('span', { class: 'kbd-combo' }, ...out);
}

function shortcuts() {
  const actions = payload.shortcut_actions || [];
  s.shortcuts = s.shortcuts || {};
  const effective = (a) => s.shortcuts[a.id] ?? a.defaults;
  const search = h('input', { type: 'search', placeholder: 'Search shortcuts', class: 'kbd-search' });
  let stopCapture = null;

  const rowFor = (a) => {
    const list = h('div', { class: 'kbd-list' });
    const edit = h('button', { class: 'icon-btn', title: 'Change' }, icon('edit'));
    const reset = h('button', { class: 'icon-btn', title: 'Reset' }, icon('reload'));
    const row = h('div', { class: 'setting kbd-row' }, h('div', { class: 'txt' }, h('div', { class: 't', text: a.label })), list, h('div', { class: 'kbd-acts' }, edit, reset));
    const draw = () => {
      const combos = effective(a);
      list.replaceChildren(...(combos.length ? combos.map(keys) : [h('span', { class: 'muted', text: 'Not set' })]));
      reset.hidden = !(a.id in s.shortcuts);
      row.classList.toggle('custom', a.id in s.shortcuts);
      row._text = `${a.label} ${i18n.tr(a.label)} ${combos.join(' ')}`.toLowerCase();
    };
    reset.addEventListener('click', () => {
      delete s.shortcuts[a.id];
      save();
      redrawAll();
    });
    edit.addEventListener('click', () => {
      if (stopCapture) stopCapture();
      row.classList.add('capturing');
      list.replaceChildren(h('span', { class: 'kbd-wait', text: 'Press a key combination… (Esc to cancel)' }));
      mizu.send('shortcut_capture', { on: true });
      const onKey = (e) => {
        e.preventDefault();
        e.stopPropagation();
        if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return;
        if (e.key === 'Escape' && !e.ctrlKey && !e.altKey && !e.shiftKey) {
          stop();
          return;
        }
        const combo = comboFromEvent(e);
        const fkey = /^F\d+$/.test(keyName(e) || '');
        if (!combo || (!e.ctrlKey && !e.altKey && !fkey)) {
          showToast('Use Ctrl or Alt with a key, or a function key (F1–F24)');
          return;
        }
        // A combination belongs to one action: take it away from any other.
        for (const other of actions) {
          if (other.id !== a.id && effective(other).some((c) => sameCombo(c, combo))) {
            s.shortcuts[other.id] = effective(other).filter((c) => !sameCombo(c, combo));
            showToast(i18n.tr('Removed from "{0}"').replace('{0}', i18n.tr(other.label)));
          }
        }
        s.shortcuts[a.id] = [combo];
        stop();
        save();
        redrawAll();
      };
      const stop = () => {
        window.removeEventListener('keydown', onKey, true);
        mizu.send('shortcut_capture', { on: false });
        row.classList.remove('capturing');
        stopCapture = null;
        draw();
      };
      stopCapture = stop;
      window.addEventListener('keydown', onKey, true);
    });
    row._draw = draw;
    draw();
    return row;
  };

  const groups = [...new Set(actions.map((a) => a.group))];
  const rows = [];
  const cards = groups.map((g) => {
    const els = actions.filter((a) => a.group === g).map(rowFor);
    rows.push(...els);
    const c = card(g, null, ...els);
    c._rows = els;
    return c;
  });
  const fixedRows = FIXED_SHORTCUTS.map(([label, combos]) => {
    const row = h('div', { class: 'setting kbd-row' }, h('div', { class: 'txt' }, h('div', { class: 't', text: label })), h('div', { class: 'kbd-list' }, ...combos.map(keys)));
    row._text = `${label} ${i18n.tr(label)} ${combos.join(' ')}`.toLowerCase();
    return row;
  });
  const fixed = card('Page', 'These are handled by the web engine and cannot be changed.', ...fixedRows);
  fixed._rows = fixedRows;
  cards.push(fixed);
  const redrawAll = () => rows.forEach((r) => r._draw());

  search.addEventListener('input', () => {
    const q = search.value.trim().toLowerCase();
    for (const c of cards) {
      let any = false;
      for (const r of c._rows) {
        r.hidden = !!q && !r._text.includes(q);
        any = any || !r.hidden;
      }
      c.hidden = !any;
    }
  });
  const resetAll = h('button', { class: 'btn' }, icon('reload'), h('span', { text: 'Reset all shortcuts' }));
  resetAll.addEventListener('click', () => {
    s.shortcuts = {};
    save();
    redrawAll();
    showToast('Shortcuts reset');
  });
  return [
    h('p', { class: 'muted kbd-note', text: 'Click the pencil next to an action and press the new keys. Changes apply immediately in every window.' }),
    h('div', { class: 'kbd-top' }, search, resetAll),
    ...cards,
  ];
}

const RENDER = { general, appearance, search, privacy, security, downloads, tabs, performance, media, shortcuts, advanced };

// ── Layout ───────────────────────────────────────────────────
const nav = h('nav', { class: 'set-nav' });
const content = h('div', { class: 'set-content' });

function renderNav() {
  nav.replaceChildren();
  for (const [id, label, ic] of CATEGORIES) {
    const b = h('button', { class: id === current ? 'on' : '' }, icon(ic), h('span', { text: label }));
    b.addEventListener('click', () => {
      current = id;
      history.replaceState(null, '', `#${id}`);
      renderNav();
      renderContent();
    });
    nav.append(b);
  }
}

function renderContent() {
  if (!s) return;
  const title = CATEGORIES.find((c) => c[0] === current) || CATEGORIES[0];
  content.replaceChildren(h('h2', { class: 'set-title', text: title[1] }), ...(RENDER[title[0]] || general)());
  content.scrollTop = 0;
}

main.append(h('div', { class: 'set' }, h('aside', null, pageHead('Settings'), nav), content));

bootPage(() => {
  mizu.request('get_settings').then((p) => {
    payload = p;
    s = structuredClone(p.settings);
    if (!RENDER[current]) current = 'general';
    renderNav();
    renderContent();
  });
});
mizu.on('toast', (m) => showToast(m.message));
mizu.on('settings', () => {
  // Another surface changed settings (e.g. bookmarks bar shortcut): refresh our copy.
  clearTimeout(refreshTimer);
  refreshTimer = setTimeout(() => {
    if (saveTimer) return; // our own pending change wins
    mizu.request('get_settings').then((p) => {
      payload = p;
      if (stripDerived(p.settings) !== stripDerived(s)) {
        s = structuredClone(p.settings);
        renderContent();
      }
    });
  }, 300);
});
let refreshTimer = 0;
