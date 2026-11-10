// Shown instead of a page that a filter list marks as dangerous or unwanted.
'use strict';

const main = document.getElementById('main');
const target = new URLSearchParams(location.search).get('url') || '';

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
