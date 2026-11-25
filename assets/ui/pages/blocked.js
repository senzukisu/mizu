// Shown instead of a page that a filter list marks as dangerous or unwanted.
'use strict';

const main = document.getElementById('main');
const params = new URLSearchParams(location.search);
const target = params.get('url') || '';

// HTTPS-First: the site did not answer over HTTPS.
if (params.get('kind') === 'http') {
  const host = (() => {
    try {
      return new URL(target).host;
    } catch {
      return target;
    }
  })();
  const back = h('button', { class: 'btn primary', text: 'Go back' });
  back.addEventListener('click', () => (history.length > 1 ? history.back() : mizu.send('open_url', { url: 'mizu://newtab/', disposition: 'current' })));
  const remember = h('input', { type: 'checkbox', checked: true });
  const go = h('button', { class: 'btn ghost', text: 'Continue to the site (not secure)' });
  go.addEventListener('click', () => mizu.request('http_proceed', { url: target, remember: remember.checked }).catch((e) => showToast(e.message)));
  main.append(
    h('div', { class: 'page blocked http' },
      h('div', { class: 'blocked-icon warn' }, icon('lock', { size: 46 })),
      h('h1', { text: 'This site has no secure version' }),
      h('p', { class: 'blocked-host', translate: 'no', text: host }),
      h('p', { text: 'It does not answer over HTTPS. Over HTTP, what you see and send can be read or changed on the way — fine for reading, not for passwords or payments.' }),
      h('p', { class: 'mono', translate: 'no', text: target }),
      h('div', { class: 'actions' }, back, go),
      h('label', { class: 'blocked-remember' }, remember, h('span', { text: 'Remember for this site' })),
      h('p', { class: 'muted', text: 'Turn HTTPS-First off or remove exceptions in Settings → Privacy.' }),
    ),
  );
  bootPage(() => (document.title = i18n.tr('This site has no secure version')));
} else {

const back = h('button', { class: 'btn primary', text: 'Go back' });
back.addEventListener('click', () => history.back());
const proceed = h('button', { class: 'btn ghost', text: 'Open anyway' });
proceed.addEventListener('click', () => mizu.send('blocked_proceed', { url: target }));

main.append(
  h('div', { class: 'page blocked' },
    h('div', { class: 'blocked-icon' }, icon('shield-off', { size: 48 })),
    h('h1', { text: 'Mizu Shield blocked this page' }),
    h('p', { text: 'A filter list you use marks this address as malicious, deceptive or unwanted.' }),
    h('p', { class: 'mono', text: target }),
    h('div', { class: 'actions' }, back, proceed),
    h('p', { class: 'muted', text: 'If you open it anyway, the site stays allowed until you close Mizu.' }),
  ),
);
bootPage();
}
