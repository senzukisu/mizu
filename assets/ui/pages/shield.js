// Mizu Shield dashboard: how much was blocked (today, this month, in total), a 30-day timeline,
// the sites where the most was blocked and the trackers blocked most often. Everything here is
// counted and kept on this device only.
'use strict';

const main = document.getElementById('main');
const fmt = (n) => Number(n || 0).toLocaleString(document.documentElement.lang || undefined);

function stat(value, label, accent) {
  return h('div', { class: `stat${accent ? ' accent' : ''}` }, h('b', { text: fmt(value) }), h('span', { text: label }));
}

// Thirty bars, one per day; the tallest fills the height. Hover shows the date and count.
function timeline(days) {
  const max = Math.max(1, ...days.map((d) => d[1]));
  const lang = document.documentElement.lang || undefined;
  const label = (iso) => new Date(`${iso}T12:00:00Z`).toLocaleDateString(lang, { day: 'numeric', month: 'short' });
  const bars = h('div', { class: 'sh-bars', role: 'img', 'aria-label': i18n.tr('Blocked per day, last 30 days') },
    ...days.map(([day, n], i) => h('div', { class: `sh-bar${i === days.length - 1 ? ' today' : ''}`, title: `${label(day)} — ${fmt(n)}` },
      h('span', { style: { height: `${n ? Math.max(4, (100 * n) / max) : 0}%` } }))));
  return h('section', { class: 'card sh-time' },
    h('div', { class: 'sh-card-head' }, h('h2', { text: 'Last 30 days' }), h('span', { class: 'muted', text: i18n.tr(`Most in a day: ${fmt(max === 1 && !days.some((d) => d[1]) ? 0 : max)}`) })),
    bars,
    h('div', { class: 'sh-axis' }, h('span', { text: label(days[0][0]) }), h('span', { text: 'Today' })));
}

function ranked(title, rows, empty, linkFor) {
  const box = h('section', { class: 'card sh-list' }, h('h2', { text: title }));
  if (!rows.length) {
    box.append(h('p', { class: 'note', text: empty }));
    return box;
  }
  const max = Math.max(1, ...rows.map((r) => r[1]));
  for (const [host, n] of rows) {
    const row = h(linkFor ? 'button' : 'div', { class: `list-row${linkFor ? ' link' : ''}` },
      letterAvatar(`https://${host}`, host),
      h('div', { class: 'main', translate: 'no' }, h('div', { class: 't', text: host }), h('div', { class: 'meter' }, h('div', { style: { width: `${(100 * n) / max}%` } }))),
      h('span', { class: 'when', text: fmt(n) }));
    if (linkFor) row.addEventListener('click', () => mizu.send('open_url', { url: linkFor(host), disposition: 'current' }));
    box.append(row);
  }
  return box;
}

async function render() {
  const s = await mizu.request('shield_stats');
  const level = { off: 'Off', basic: 'Basic', balanced: 'Balanced', strict: 'Strict' }[s.tracking_prevention] || s.tracking_prevention;
  const days = s.days || [];
  const today = days.length ? days[days.length - 1][1] : 0;

  const settingsBtn = h('button', { class: 'btn', text: 'Shield settings' });
  settingsBtn.addEventListener('click', () => mizu.send('open_url', { url: 'mizu://settings/#shield', disposition: 'current' }));
  const update = h('button', { class: 'btn', text: 'Update filter lists' });
  update.addEventListener('click', () => {
    mizu.send('filters_update', { force: true });
    update.disabled = true;
    update.textContent = i18n.tr('Updating…');
  });
  const pill = (on, text) => h('span', { class: `pill ${on ? 'ok' : 'bad'}`, text });

  main.replaceChildren(
    h('div', { class: 'page sh' },
      pageHead('Mizu Shield', update, settingsBtn),
      h('div', { class: 'stats' },
        stat(today, 'blocked today', true),
        stat(s.month, 'in the last 30 days'),
        stat(s.lifetime, 'blocked in total'),
        stat(s.rules, 'active filter rules')),
      days.length ? timeline(days) : null,
      h('div', { class: 'sh-cols' },
        ranked('Sites where the most was blocked', s.sites || [], 'Nothing blocked yet. Sites you visit appear here with how much Shield stopped on them.', (host) => `mizu://site/?host=${encodeURIComponent(host)}`),
        ranked('Blocked most often this session', s.top || [], 'Nothing blocked yet in this session.')),
      h('section', { class: 'card' },
        h('h2', { text: 'Protection' }),
        h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'Ad & tracker blocking' }), h('div', { class: 'd', text: 'uBlock Origin–compatible filtering: network rules, element hiding, scriptlets.' })), pill(s.enabled, s.enabled ? 'On' : 'Off')),
        h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'HTTPS-First' }), h('div', { class: 'd', text: 'Sites open over a secure connection when they have one.' })), pill(s.https_first, s.https_first ? 'On' : 'Off')),
        h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'Engine tracking prevention' }), h('div', { class: 'd', text: 'Restricts storage access for known trackers.' })), h('span', { class: 'pill accent', text: level })),
        h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'Sites where Shield is off' })), h('span', { class: 'pill', text: String(s.allowlist.length) }))),
      h('p', { class: 'muted', text: 'Statistics are kept on this device only. Mizu Shield cannot make you anonymous: websites can still see your IP address and recognise you when you sign in.' }),
    ),
  );
}

mizu.on('toast', (m) => {
  showToast(m.message);
  render();
});
bootPage(render);
