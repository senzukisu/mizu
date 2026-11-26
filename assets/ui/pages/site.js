// Site settings (mizu://site/?host=example.com): one page with everything Mizu does and
// keeps for one site: Shield, cookies and storage, "keep me signed in", permissions, history.
'use strict';

const main = document.getElementById('main');
const host = (new URLSearchParams(location.search).get('host') || '').trim().toLowerCase();
let info = null;

const PERM = { camera: ['camera', 'Camera'], microphone: ['mic', 'Microphone'], geolocation: ['location', 'Location'], notifications: ['bell', 'Notifications'], clipboard_read: ['clipboard', 'Clipboard'] };

function card(title, note, ...rows) {
  return h('section', { class: 'card' }, title ? h('h2', { text: title }) : null, note ? h('p', { class: 'note', text: note }) : null, ...rows.filter(Boolean));
}
function row(title, desc, control) {
  return h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: title }), desc ? h('div', { class: 'd', text: desc }) : null), control);
}

function cookiesCard() {
  const count = h('div', { class: 'd', text: 'Reading…' });
  const list = h('div', { class: 'site-cookies' });
  const forget = h('button', { class: 'btn danger', text: 'Delete cookies and site data' });
  forget.addEventListener('click', async () => {
    if (!confirm(i18n.tr('Delete this site’s cookies and data? You will be signed out of it.'))) return;
    forget.disabled = true;
    const r = await mizu.request('site_forget', { host }).catch(() => null);
    showToast(i18n.tr(r ? 'Site data deleted' : 'Could not delete site data'));
    forget.disabled = false;
    loadCookies();
  });
  const loadCookies = () =>
    mizu.request('site_cookies_for', { host }).then((r) => {
      const cookies = (r && r.cookies) || [];
      count.textContent = cookies.length === 1 ? i18n.tr('1 cookie') : i18n.tr(`${cookies.length} cookies`);
      list.replaceChildren(...cookies.slice(0, 60).map((c) => h('div', { translate: 'no' }, h('b', { text: c.name }), h('span', { text: ` · ${c.domain}${c.session ? ' · session' : ''}` }))));
    });
  loadCookies();
  const keep = switchInput(info.keep_signed_in, (v) => mizu.request('site_keep_signed_in', { host, keep: v }), { label: 'Keep me signed in' });
  const cleanupNote = info.cleanup_days
    ? `Mizu deletes cookies of sites you haven’t visited for ${info.cleanup_days} days. Turn this on to keep this site signed in.`
    : 'Exempts the site from automatic cookie cleanup (Settings → Privacy).';
  return card('Cookies and site data', null,
    h('div', { class: 'setting col' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'Cookies' }), count), list),
    row('Keep me signed in', cleanupNote, keep),
    row('Forget this site', 'Deletes its cookies, local storage, IndexedDB, caches and service workers. History and bookmarks stay.', forget));
}

function shieldCard() {
  const toggle = switchInput(info.shield, (v) => mizu.request('site_shield', { host, enabled: v }).then(() => showToast(i18n.tr('Reload the site to apply'))), { label: 'Mizu Shield', disabled: !info.shield_global });
  return card('Mizu Shield', null,
    row('Block ads and trackers on this site', info.shield_global ? 'Turn off only if the site does not work with Shield.' : 'Content blocking is off for all sites in Settings → Privacy.', toggle),
    row('Third-party cookies', info.third_party_blocked ? 'Blocked for every site (Settings → Privacy).' : 'Allowed (Settings → Privacy).', h('span', { class: `pill${info.third_party_blocked ? ' ok' : ''}`, text: info.third_party_blocked ? 'Blocked' : 'Allowed' })));
}

function permissionsCard() {
  if (!info.permissions.length) return card('Permissions', 'The site has not asked for any permission, or you have not answered yet.');
  const rows = info.permissions.map((p) => {
    const [ic, label] = PERM[p.kind] || ['key', p.kind.replace(/_/g, ' ')];
    const sel = h('select', null, ...[['allow', 'Allow'], ['block', 'Block'], ['reset', 'Ask']].map(([v, t]) => {
      const o = h('option', { value: v, text: t });
      if (p.state === v) o.selected = true;
      return o;
    }));
    sel.addEventListener('change', () => mizu.request('permission_set', { origin: p.origin, kind: p.kind, state: sel.value }).then(() => showToast(i18n.tr('Saved'))));
    return h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't' }, icon(ic, { size: 15 }), h('span', { style: { marginLeft: '8px' }, text: label })), h('div', { class: 'd', translate: 'no', text: p.origin })), sel);
  });
  return card('Permissions', null, ...rows);
}

function fingerprintSiteCard() {
  if (!info.fingerprinting) return null;
  const toggle = switchInput(!info.fp_excepted, (v) => mizu.request('site_fingerprint', { host, enabled: v }).then(() => showToast(i18n.tr('Reload the site to apply'))), { label: 'Fingerprinting defence' });
  return card('Fingerprinting defence', null, row('Protect this site', info.fp_level === 'strict' ? 'Strict: gives the site a generic graphics card, a common screen size and slightly different canvas and audio. Turn off only if the site does not work.' : 'Gives the site slightly different canvas, audio and hardware values. Turn off only if the site does not work.', toggle));
}

function httpCard() {
  if (!info.http_allowed) return null;
  const forget = h('button', { class: 'btn', text: 'Use HTTPS again' });
  forget.addEventListener('click', () => mizu.request('http_forget', { host }).then(() => {
    info.http_allowed = false;
    render();
  }));
  return card('Secure connection', null, row('Allowed without HTTPS', 'You chose to open this site over HTTP when it had no secure version.', forget));
}

function zoomCard() {
  const pct = info.zoom || info.zoom_default;
  const reset = h('button', { class: 'btn', text: 'Reset' });
  reset.disabled = !info.zoom;
  reset.addEventListener('click', () => mizu.request('site_zoom_reset', { host }).then(() => {
    info.zoom = null;
    render();
  }));
  return card('Zoom', null,
    row(i18n.tr(`Zoom: ${pct}%`), info.zoom ? 'Mizu remembers the zoom you choose for each site (Ctrl+plus, Ctrl+minus, Ctrl+wheel).' : 'The default zoom (Settings → Appearance). Zoom in on the site and Mizu remembers it here.', reset));
}

function historyCard() {
  const when = info.last_visit ? new Date(info.last_visit).toLocaleString(document.documentElement.lang || undefined, { dateStyle: 'medium', timeStyle: 'short' }) : null;
  const open = h('button', { class: 'btn', text: 'Open history' });
  open.addEventListener('click', () => mizu.send('open_url', { url: 'mizu://history/', disposition: 'current' }));
  return card('History', null, row(when ? i18n.tr(`Last visit: ${when}`) : i18n.tr('Not in your history'), info.visits ? i18n.tr(`${info.visits} visits`) : null, open));
}

function render() {
  const go = h('button', { class: 'btn primary', text: 'Open site' });
  go.addEventListener('click', () => mizu.send('open_url', { url: `https://${host}/`, disposition: 'current' }));
  main.replaceChildren(h('div', { class: 'page', style: { maxWidth: '820px' } },
    h('div', { class: 'site-head' }, letterAvatar(`https://${host}/`, host), h('div', { class: 'grow' }, h('h1', { translate: 'no', text: host }), h('div', { class: 'd', text: 'Site settings' })), go),
    ...[shieldCard(), fingerprintSiteCard(), httpCard(), cookiesCard(), permissionsCard(), zoomCard(), historyCard()].filter(Boolean)));
}

bootPage(() => {
  if (!host) {
    main.replaceChildren(h('div', { class: 'page' }, h('div', { class: 'empty-state' }, h('div', { text: 'No site chosen.' }))));
    return;
  }
  document.title = `${host} – ${i18n.tr('Site settings')}`;
  mizu.request('site_get', { host }).then((d) => {
    info = d;
    render();
  }).catch((e) => main.replaceChildren(h('div', { class: 'page' }, h('div', { class: 'empty-state' }, h('div', { text: e.message })))));
});
