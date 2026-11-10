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

async function installView() {
  const p = await mizu.request('script_pending');
  if (!p) {
    history.replaceState(null, '', '#');
    return null;
  }
  const m = p.meta;
  const install = h('button', { class: 'btn primary', text: p.installed_version ? `Update to ${m.version || 'new version'}` : 'Install' });
  install.addEventListener('click', () => mizu.request('script_install_confirm').then(() => {
    showToast(`${m.name} installed`);
    history.replaceState(null, '', '#');
    render();
  }).catch((e) => showToast(e.message)));
  const cancel = h('button', { class: 'btn', text: 'Cancel' });
  cancel.addEventListener('click', () => {
    mizu.send('script_install_cancel');
    history.replaceState(null, '', '#');
    render();
  });
  const where = [...m.matches, ...m.includes];
  const unsupported = m.grants.filter((g) => !/^(none|GM[_.](info|getValue|setValue|deleteValue|listValues|addStyle|addElement|openInTab|setClipboard|log|registerMenuCommand|unregisterMenuCommand|notification|xmlhttpRequest|xmlHttpRequest)|unsafeWindow|window\.(close|focus|onurlchange))$/.test(g));
  return h('section', { class: 'card install' },
    h('h2', { text: p.installed_version ? 'Update userscript' : 'Install userscript' }),
    h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: `${m.name} ${m.version}` }), h('div', { class: 'd', text: m.description || 'No description' }), h('div', { class: 'd', text: `From ${hostOf(p.url) || p.url}${m.author ? ` · by ${m.author}` : ''}` })), cancel, install),
    h('div', { class: 'setting col' },
      h('div', { class: 'txt' }, h('div', { class: 't', text: 'Runs on' })),
      h('div', { class: 'chips' }, ...(where.length ? where : ['(no pages — the script will never run)']).map((w) => h('span', { class: 'pill mono', text: w }))),
      p.requires_bytes ? h('div', { class: 'd', text: `Includes ${formatBytes(p.requires_bytes)} of @require libraries.` }) : null,
      unsupported.length ? h('div', { class: 'd warn', text: `Uses features Mizu Scripts does not support yet: ${unsupported.join(', ')}. Parts of the script may not work.` }) : null,
      h('div', { class: 'd warn', text: 'Userscripts can read and change everything on the pages they run on. Install only scripts you trust.' })),
    h('details', null, h('summary', { text: 'Show source code' }), h('pre', { class: 'code', text: p.source })),
  );
}

async function scriptsView() {
  const scripts = await mizu.request('scripts_get');
  const card = h('section', { class: 'card' });
  if (!scripts.length) {
    card.append(h('div', { class: 'empty-state' }, icon('code'), h('div', { text: 'No userscripts yet.' }), h('div', { class: 'muted', text: 'Open any .user.js link (for example on greasyfork.org or openuserjs.org) and Mizu will offer to install it.' })));
  }
  for (const s of scripts) {
    const m = s.meta;
    const view = h('button', { class: 'icon-btn', title: 'View source' }, icon('eye'));
    view.addEventListener('click', () => mizu.request('script_source', { id: s.id }).then((r) => sourceDialog(m.name, r.source)));
    const del = h('button', { class: 'icon-btn', title: 'Remove' }, icon('trash'));
    del.addEventListener('click', () => {
      if (confirm(`Remove "${m.name}"?`)) mizu.request('script_remove', { id: s.id }).then(render);
    });
    const sw = switchInput(s.enabled, (v) => mizu.request('script_toggle', { id: s.id, enabled: v }).then(() => showToast(v ? 'Enabled — reload pages to apply' : 'Disabled')));
    card.append(h('div', { class: 'list-row' },
      h('div', { class: 'fav' }, icon('code')),
      h('div', { class: 'main' }, h('div', { class: 't', text: `${m.name}  ${m.version}` }), h('div', { class: 'u', text: m.description || [...m.matches, ...m.includes].join(', ') })),
      h('div', { class: 'acts' }, view, del), sw));
  }
  const create = h('button', { class: 'btn', text: 'New script' });
  create.addEventListener('click', createDialog);
  return [card, h('div', { class: 'row-actions' }, create), h('p', { class: 'muted', text: 'Mizu Scripts understands the Greasemonkey/Tampermonkey format (@match, @include, @exclude, @require, @run-at) and the common GM_* API. GM_xmlhttpRequest follows normal web CORS rules.' })];
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
  if (location.hash === '#install') {
    const iv = await installView();
    if (iv) page.append(iv);
  }
  page.append(...(tab === 'scripts' ? await scriptsView() : await extensionsView()));
  main.replaceChildren(page);
}

bootPage(render);
