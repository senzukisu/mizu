// Mizu Scripts (userscripts) and browser extensions.
'use strict';

const main = document.getElementById('main');
let tab = location.hash === '#extensions' ? 'extensions' : 'scripts';

const TEMPLATE = `// ==UserScript==
// @name         My script
// @namespace    https://example.com/
// @version      1.0
// @description  Describe what it does
// @match        https://example.com/*
// @grant        none
// @run-at       document-end
// ==/UserScript==

(function () {
  'use strict';
  console.log('Hello from Mizu Scripts');
})();
`;

function sourceDialog(title, source) {
  const pre = h('pre', { class: 'code', text: source });
  const dlg = h('dialog', { style: { width: 'min(900px, 92vw)' } }, h('h3', { text: title }), pre, h('div', { class: 'actions' }, h('button', { class: 'btn', text: 'Close', onclick: () => dlg.close() })));
  document.body.append(dlg);
  dlg.showModal();
  dlg.addEventListener('close', () => dlg.remove());
}

function createDialog() {
  const ta = h('textarea', { rows: 18, spellcheck: 'false' });
  ta.value = TEMPLATE;
  const dlg = h('dialog', { style: { width: 'min(860px, 92vw)' } }, h('h3', { text: 'New userscript' }), ta,
    h('div', { class: 'actions' }, h('button', { class: 'btn', text: 'Cancel', onclick: () => dlg.close() }), h('button', { class: 'btn primary', text: 'Save', onclick: () => {
      mizu.request('script_create', { source: ta.value }).then(() => {
        dlg.close();
        showToast('Script saved');
        render();
      }).catch((e) => showToast(e.message));
    } })));
  ta.style.width = '100%';
  document.body.append(dlg);
  dlg.showModal();
  dlg.addEventListener('close', () => dlg.remove());
}

// ── Where a script runs ─────────────────────────────────────
// A small list editor: type a site and press Enter (or paste several, one per line).
function siteList(values, placeholder) {
  const items = [...values];
  const chips = h('div', { class: 'site-chips' });
  const input = h('input', { type: 'text', placeholder, spellcheck: 'false' });
  const draw = () => {
    chips.replaceChildren(...items.map((v, i) => {
      const x = h('button', { class: 'x', title: 'Remove' }, icon('close', { size: 12 }));
      x.addEventListener('click', () => {
        items.splice(i, 1);
        draw();
      });
      return h('span', { class: 'site-chip', translate: 'no' }, h('span', { text: v }), x);
    }));
    chips.hidden = !items.length;
  };
  const add = () => {
    for (const raw of input.value.split(/[\n,;\s]+/)) {
      const v = cleanSite(raw);
      if (v && !items.some((x) => x.toLowerCase() === v.toLowerCase())) items.push(v);
    }
    input.value = '';
    draw();
  };
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      add();
    }
  });
  input.addEventListener('paste', () => setTimeout(add, 0));
  const plus = h('button', { class: 'btn', text: 'Add' });
  plus.addEventListener('click', add);
  draw();
  const el = h('div', { class: 'site-list' }, h('div', { class: 'site-add' }, input, plus), chips);
  return { el, input, get: () => (input.value.trim() && add(), items) };
}

// "https://www.youtube.com/watch?v=1" → "youtube.com"; patterns stay as typed.
function cleanSite(raw) {
  const v = raw.trim();
  if (!v) return '';
  if (/[*<>]/.test(v) || (v.startsWith('/') && v.endsWith('/'))) return v;
  try {
    const u = new URL(v.includes('://') ? v : `https://${v}`);
    const host = u.hostname.replace(/^www\./, '');
    const path = u.pathname !== '/' ? u.pathname.replace(/\/$/, '') : '';
    return host + path;
  } catch {
    return v;
  }
}

function sitesSummary(s) {
  const parts = [];
  if (s.sites.only.length) parts.push(`Only on ${s.sites.only.join(', ')}`);
  if (s.sites.never.length) parts.push(`Never on ${s.sites.never.join(', ')}`);
  return parts.join(' · ');
}

function sitesDialog(s) {
  const m = s.meta;
  const own = [...m.matches, ...m.includes];
  const onlyList = siteList(s.sites.only, 'youtube.com');
  const neverList = siteList(s.sites.never, 'example.com');
  let mode = s.sites.only.length ? 'only' : 'script';
  const ownBox = h('div', { class: 'site-own', translate: 'no' }, ...(own.length ? own.slice(0, 8).map((p) => h('code', { text: p })) : [h('span', { class: 'muted', text: 'The script does not list any sites.' })]), own.length > 8 ? h('span', { class: 'muted', text: `+${own.length - 8}` }) : null);
  const seg = h('div', { class: 'seg' });
  const opt = (key, label) => {
    const b = h('button', { text: label });
    b.addEventListener('click', () => {
      mode = key;
      sync();
      if (key === 'only') onlyList.input.focus();
    });
    b.dataset.k = key;
    return b;
  };
  seg.append(opt('script', 'Sites from the script'), opt('only', 'Only sites I choose'));
  const onlyBox = h('div', { class: 'site-block' }, onlyList.el);
  const sync = () => {
    for (const b of seg.children) b.classList.toggle('on', b.dataset.k === mode);
    ownBox.hidden = mode !== 'script';
    onlyBox.hidden = mode !== 'only';
  };
  sync();

  const save = h('button', { class: 'btn primary', text: 'Save' });
  const dlg = h('dialog', { class: 'sites-dialog' },
    h('h3', { text: `Where "${m.name}" runs` }),
    h('div', { class: 'site-section' },
      h('div', { class: 'site-label', text: 'Run on' }),
      seg, ownBox, onlyBox),
    h('div', { class: 'site-section' },
      h('div', { class: 'site-label', text: 'Never run on' }),
      neverList.el),
    h('p', { class: 'muted site-hint', text: 'A site name covers all its pages and subdomains: youtube.com. Limit to a section with a path: github.com/user. Patterns like https://*.example.org/* work too.' }),
    h('div', { class: 'actions' }, h('button', { class: 'btn', text: 'Cancel', onclick: () => dlg.close() }), save));
  save.addEventListener('click', () => {
    const only = mode === 'only' ? onlyList.get() : [];
    const never = neverList.get();
    if (mode === 'only' && !only.length) {
      showToast('Add at least one site');
      onlyList.input.focus();
      return;
    }
    mizu.request('script_sites', { id: s.id, only, never }).then(() => {
      dlg.close();
      showToast('Saved — reload pages to apply');
      render();
    }).catch((e) => showToast(e.message));
  });
  document.body.append(dlg);
  dlg.showModal();
  dlg.addEventListener('close', () => dlg.remove());
}

const SUPPORTED_GRANT = /^(none|GM[_.](info|getValue|setValue|deleteValue|listValues|addStyle|addElement|openInTab|setClipboard|log|registerMenuCommand|unregisterMenuCommand|notification|xmlhttpRequest|xmlHttpRequest)|unsafeWindow|window\.(close|focus|onurlchange))$/;

async function installView() {
  const p = await mizu.request('script_pending');
  if (!p || (!p.items.length && !p.failures.length)) {
    if (location.hash === '#install') history.replaceState(null, '', '#');
    return null;
  }
  const card = h('section', { class: 'card install' }, h('h2', { text: p.items.length > 1 ? `Install ${p.items.length} userscripts` : p.items.length ? (p.items[0].installed_version ? 'Update userscript' : 'Install userscript') : 'Install userscripts' }));
  const checks = [];
  for (const it of p.items) {
    const m = it.meta;
    const cb = h('input', { type: 'checkbox', checked: true });
    checks.push([it.index, cb]);
    const where = [...m.matches, ...m.includes];
    const unsupported = m.grants.filter((g) => !SUPPORTED_GRANT.test(g));
    const badge = it.installed_version
      ? h('span', { class: 'pill accent', text: `update ${it.installed_version} → ${m.version || '?'}` })
      : h('span', { class: 'pill ok', text: 'new' });
    card.append(
      h('div', { class: 'setting col' },
        h('label', { class: 'pick' }, cb, h('div', { class: 'txt' },
          h('div', { class: 't' }, `${m.name} ${m.version}  `, badge),
          h('div', { class: 'd', text: m.description || 'No description' }),
          h('div', { class: 'd', text: `From ${hostOf(it.url) || it.url}${m.author ? ` · by ${m.author}` : ''}${it.requires_bytes ? ` · ${formatBytes(it.requires_bytes)} of @require libraries` : ''}` }))),
        h('div', { class: 'chips' }, ...(where.length ? where.slice(0, 8) : ['(no pages — the script will never run)']).map((w) => h('span', { class: 'pill mono', text: w })), where.length > 8 ? h('span', { class: 'pill', text: `+${where.length - 8}` }) : null),
        unsupported.length ? h('div', { class: 'd warn', text: `Uses features Mizu Scripts does not support yet: ${unsupported.join(', ')}.` }) : null,
        h('details', null, h('summary', { text: 'Show source code' }), h('pre', { class: 'code', text: it.source })),
      ),
    );
  }
  for (const f of p.failures) {
    card.append(h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: f.url }), h('div', { class: 'd warn', text: f.error }))));
  }
  const cancel = h('button', { class: 'btn', text: 'Cancel' });
  cancel.addEventListener('click', () => {
    mizu.send('script_install_cancel');
    history.replaceState(null, '', '#');
    render();
  });
  const install = h('button', { class: 'btn primary', text: p.items.length > 1 ? 'Install selected' : 'Install', disabled: !p.items.length });
  install.addEventListener('click', () => {
    const indices = checks.filter(([, cb]) => cb.checked).map(([i]) => i);
    if (!indices.length) return showToast('Nothing selected');
    mizu.request('script_install_confirm', { indices }).then((r) => {
      showToast(r.installed.length === 1 ? `${r.installed[0]} installed` : `${r.installed.length} scripts installed`);
      history.replaceState(null, '', '#');
      render();
    }).catch((e) => showToast(e.message));
  });
  card.append(h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 'd warn', text: 'Userscripts can read and change everything on the pages they run on. Install only scripts you trust.' })), cancel, install));
  return card;
}

function addScriptsCard() {
  const links = h('textarea', { rows: 3, spellcheck: 'false', placeholder: 'One link per line: GitHub, GitLab, Gist, Codeberg, Greasy Fork, OpenUserJS or any .user.js URL' });
  links.style.width = '100%';
  const fetchBtn = h('button', { class: 'btn primary', text: 'Get scripts' });
  fetchBtn.addEventListener('click', () => {
    const urls = links.value.split(/\s+/).filter(Boolean);
    if (!urls.length) return showToast('Paste at least one link');
    fetchBtn.disabled = true;
    fetchBtn.textContent = 'Downloading…';
    mizu.request('script_install_urls', { urls }).catch((e) => {
      showToast(e.message);
      fetchBtn.disabled = false;
      fetchBtn.textContent = 'Get scripts';
    });
  });
  const files = h('button', { class: 'btn', text: 'From files…' });
  files.addEventListener('click', () => mizu.send('script_install_files'));
  return h('section', { class: 'card' },
    h('h2', { text: 'Add scripts' }),
    h('p', { class: 'note', text: 'Clicking a .user.js link anywhere installs it too. Several links or files can be added at once.' }),
    h('div', { class: 'setting col' }, links, h('div', { class: 'row-actions', style: { margin: '0' } }, files, fetchBtn)));
}

async function scriptsView() {
  const scripts = await mizu.request('scripts_get');
  const card = h('section', { class: 'card' });
  if (!scripts.length) {
    card.append(h('div', { class: 'empty-state' }, icon('code'), h('div', { text: 'No userscripts yet.' }), h('div', { class: 'muted', text: 'Open any .user.js link (for example on greasyfork.org or openuserjs.org) and Mizu will offer to install it.' })));
  }
  for (const s of scripts) {
    const m = s.meta;
    const sites = h('button', { class: 'icon-btn', title: 'Where it runs' }, icon('globe'));
    sites.addEventListener('click', () => sitesDialog(s));
    const view = h('button', { class: 'icon-btn', title: 'View source' }, icon('eye'));
    view.addEventListener('click', () => mizu.request('script_source', { id: s.id }).then((r) => sourceDialog(m.name, r.source)));
    const del = h('button', { class: 'icon-btn', title: 'Remove' }, icon('trash'));
    del.addEventListener('click', () => {
      if (confirm(`Remove "${m.name}"?`)) mizu.request('script_remove', { id: s.id }).then(render);
    });
    const sw = switchInput(s.enabled, (v) => mizu.request('script_toggle', { id: s.id, enabled: v }).then(() => showToast(v ? 'Enabled — reload pages to apply' : 'Disabled')));
    const source = s.source_url || m.download_url;
    const sub = source ? `${hostOf(source) || 'local file'}${s.last_checked ? ` · checked ${relativeTime(s.last_checked)}` : ''}` : 'Local script · no automatic updates';
    card.append(h('div', { class: 'list-row' },
      h('div', { class: 'fav' }, icon('code')),
      h('div', { class: 'main' }, h('div', { class: 't', text: `${m.name}  ${m.version}` }), h('div', { class: 'u', text: m.description || [...m.matches, ...m.includes].join(', ') }), h('div', { class: 'u', text: sub }),
        sitesSummary(s) ? h('button', { class: 'site-pill', title: 'Where it runs', onclick: () => sitesDialog(s) }, icon('globe', { size: 12 }), h('span', { translate: 'no', text: sitesSummary(s) })) : null),
      h('div', { class: 'acts' }, sites, view, del), sw));
  }
  const create = h('button', { class: 'btn', text: 'New script' });
  create.addEventListener('click', createDialog);
  const updates = h('button', { class: 'btn', text: 'Check for updates', disabled: !scripts.length });
  updates.addEventListener('click', () => {
    updates.disabled = true;
    updates.textContent = 'Checking…';
    mizu.request('scripts_check_updates').then((r) => {
      showToast(r.updated ? `${r.updated} updated: ${r.names.join(', ')}` : r.failed ? `No updates (${r.failed} could not be checked)` : 'All scripts are up to date');
      render();
    }).catch((e) => showToast(e.message)).finally(() => {
      updates.disabled = false;
      updates.textContent = 'Check for updates';
    });
  });
  return [addScriptsCard(), card, h('div', { class: 'row-actions' }, updates, create), h('p', { class: 'muted', text: 'Mizu Scripts understands the Greasemonkey/Tampermonkey format (@match, @include, @exclude, @require, @run-at) and the common GM_* API. Scripts with an update source are checked for new versions once a day (Settings → Advanced). GM_xmlhttpRequest follows normal web CORS rules.' })];
}

async function extensionsView() {
  const card = h('section', { class: 'card' });
  const install = h('button', { class: 'btn primary', text: 'Install extension…' });
  install.addEventListener('click', () => mizu.request('extension_install').then((r) => {
    if (r) {
      showToast(`${r.name} installed`);
      render();
    }
  }).catch((e) => showToast(e.message)));
  try {
    const list = await mizu.request('extensions_get');
    if (!list.length) card.append(h('div', { class: 'empty-state' }, icon('puzzle'), h('div', { text: 'No extensions installed.' })));
    for (const e of list) {
      const opts = h('button', { class: 'icon-btn', title: 'Options' }, icon('settings'));
      opts.addEventListener('click', () => mizu.send('extension_open', { id: e.id }));
      const del = h('button', { class: 'icon-btn', title: 'Remove' }, icon('trash'));
      del.addEventListener('click', () => {
        if (confirm(`Remove "${e.name}"?`)) mizu.request('extension_remove', { id: e.id }).then(render);
      });
      const sw = switchInput(e.enabled, (v) => mizu.request('extension_toggle', { id: e.id, enabled: v }));
      card.append(h('div', { class: 'list-row' }, h('div', { class: 'fav' }, icon('puzzle')), h('div', { class: 'main' }, h('div', { class: 't', text: `${e.name}${e.version ? `  ${e.version}` : ''}` }), h('div', { class: 'u mono', text: e.id })), h('div', { class: 'acts' }, e.has_options ? opts : null, del), sw));
    }
  } catch (err) {
    card.append(h('div', { class: 'empty-state' }, icon('puzzle'), h('div', { text: err.message })));
  }
  const link = h('input', { type: 'url', placeholder: 'Chrome Web Store or Edge Add-ons link, e.g. https://chromewebstore.google.com/detail/…', style: { flex: '1' } });
  const fromStore = h('button', { class: 'btn primary', text: 'Add from store' });
  const doStore = () => {
    if (!link.value.trim()) return;
    fromStore.disabled = true;
    fromStore.textContent = 'Downloading…';
    mizu.request('extension_install_from_store', { url: link.value.trim() }).then((r) => {
      showToast(`${r.name} installed`);
      render();
    }).catch((e) => showToast(e.message)).finally(() => {
      fromStore.disabled = false;
      fromStore.textContent = 'Add from store';
    });
  };
  fromStore.addEventListener('click', doStore);
  link.addEventListener('keydown', (e) => e.key === 'Enter' && doStore());
  const storeCard = h('section', { class: 'card' },
    h('h2', { text: 'Install from a web store' }),
    h('p', { class: 'note', text: 'Open an extension page on chromewebstore.google.com or microsoftedge.microsoft.com and press "Add to Mizu" in the address bar — or paste its link here.' }),
    h('div', { class: 'setting' }, link, fromStore));
  install.className = 'btn';
  install.textContent = 'Install from file…';
  return [
    storeCard,
    card,
    h('div', { class: 'row-actions' }, install),
    h('p', { class: 'muted', text: 'Mizu runs Chromium Manifest V3 extensions through its engine. Choose an unpacked extension folder\'s manifest.json, or a .zip / .crx file. Extensions have no toolbar button in Mizu 0.1 — open their options from this page. Example: uBlock Origin Lite (download the Chromium .zip from its GitHub releases page). Mizu Shield already blocks ads without any extension.' }),
  ];
}

async function render() {
  const tabs = h('div', { class: 'seg' });
  for (const [id, label] of [['scripts', 'Userscripts'], ['extensions', 'Extensions']]) {
    const b = h('button', { class: id === tab ? 'on' : '', text: label });
    b.addEventListener('click', () => {
      tab = id;
      history.replaceState(null, '', id === 'extensions' ? '#extensions' : '#');
      render();
    });
    tabs.append(b);
  }
  const page = h('div', { class: 'page' }, pageHead('Scripts & extensions', tabs));
  const iv = await installView();
  if (iv) {
    page.append(iv);
    if (tab !== 'scripts') tab = 'scripts';
  }
  page.append(...(tab === 'scripts' ? await scriptsView() : await extensionsView()));
  main.replaceChildren(page);
}

mizu.on('scripts_pending', () => {
  tab = 'scripts';
  history.replaceState(null, '', '#install');
  render();
});
bootPage(render);
