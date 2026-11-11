// Mizu Shield dashboard: what was blocked and how protection is configured.
'use strict';

const main = document.getElementById('main');

function stat(value, label) {
  return h('div', { class: 'stat' }, h('b', { text: Number(value || 0).toLocaleString() }), h('span', { text: label }));
}

async function render() {
  const s = await mizu.request('shield_stats');
  const level = { off: 'Off', basic: 'Basic', balanced: 'Balanced', strict: 'Strict' }[s.tracking_prevention] || s.tracking_prevention;
  const top = h('section', { class: 'card' }, h('h2', { text: 'Most blocked this session' }));
  if (!s.top.length) top.append(h('p', { class: 'note', text: 'Nothing blocked yet in this session.' }));
  const max = Math.max(1, ...s.top.map((t) => t[1]));
  for (const [host, n] of s.top) {
    top.append(h('div', { class: 'list-row' }, letterAvatar(`https://${host}`, host), h('div', { class: 'main', translate: 'no' }, h('div', { class: 't', text: host }), h('div', { class: 'meter' }, h('div', { style: { width: `${(100 * n) / max}%` } }))), h('span', { class: 'when', text: String(n) })));
  }
  const settingsBtn = h('button', { class: 'btn', text: 'Shield settings' });
  settingsBtn.addEventListener('click', () => mizu.send('open_url', { url: 'mizu://settings/#privacy', disposition: 'current' }));
  const update = h('button', { class: 'btn', text: 'Update filter lists' });
  update.addEventListener('click', () => {
    mizu.send('filters_update', { force: true });
    update.disabled = true;
    update.textContent = 'Updating…';
  });
  main.replaceChildren(
    h('div', { class: 'page' },
      pageHead('Mizu Shield', update, settingsBtn),
      h('div', { class: 'stats' }, stat(s.session, 'blocked since Mizu started'), stat(s.lifetime, 'blocked in total'), stat(s.rules, 'active filter rules')),
      h('section', { class: 'card' },
        h('h2', { text: 'Protection' }),
        h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'Ad & tracker blocking' }), h('div', { class: 'd', text: 'uBlock Origin–compatible filtering: network rules, element hiding, scriptlets.' })), h('span', { class: `pill ${s.enabled ? 'ok' : 'bad'}`, text: s.enabled ? 'On' : 'Off' })),
        h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'Engine tracking prevention' }), h('div', { class: 'd', text: 'Restricts storage access for known trackers.' })), h('span', { class: 'pill accent', text: level })),
        h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'Sites where Shield is off' })), h('span', { class: 'pill', text: String(s.allowlist.length) }))),
      top,
      h('p', { class: 'muted', text: 'Statistics are kept on this device only. Mizu Shield cannot make you anonymous: websites can still see your IP address and recognise you when you sign in.' }),
    ),
  );
}

mizu.on('toast', (m) => {
  showToast(m.message);
  render();
});
bootPage(render);
