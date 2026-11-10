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
  return [
    card('On startup', null,
      setting('When Mizu starts', null, seg([['restore_session', 'Continue where I left off'], ['new_tab', 'New tab'], ['home_page', 'Home page']], g.startup, (v) => {
        g.startup = v;
        save();
      })),
      setting('Home page', 'Used by the Home button and on startup.', home),
      toggle('Offer to restore tabs after a crash', 'If Mizu didn\'t close normally, your tabs come back.', () => g.restore_after_crash, (v) => (g.restore_after_crash = v)),
      toggle('Confirm before closing many tabs', null, () => g.confirm_close_multiple_tabs, (v) => (g.confirm_close_multiple_tabs = v)),
    ),
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
    card('Forms and passwords', 'Mizu does not have its own password manager yet. These let the engine remember entries inside your profile.',
      toggle('Offer to save passwords', null, () => p.engine_password_saving, (v) => (p.engine_password_saving = v)),
      toggle('Autofill forms', null, () => p.engine_autofill, (v) => (p.engine_autofill = v)),
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
      setting('Userscripts and extensions', null, scripts),
    ),
    card('Engine', null, setting('Microsoft Edge WebView2', 'Updated automatically by Windows, independently of Mizu.', h('span', { class: 'mono', text: payload.engine_version || 'unknown' }))),
    card('Reset', null, setting('Restore default settings', null, reset)),
  ];
}

const RENDER = { general, appearance, search, privacy, security, downloads, tabs, performance, media, advanced };

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
