// Mizu Passwords: saved logins, editing, import/export.
'use strict';

const main = document.getElementById('main');
const list = h('section', { class: 'card' });
const neverCard = h('section', { class: 'card' });
let query = '';

function editDialog(login) {
  const site = h('input', { type: 'url', value: login ? login.origin : '', placeholder: 'https://example.com' });
  const user = h('input', { type: 'text', value: login ? login.username : '', placeholder: 'Username or email', spellcheck: 'false' });
  const pass = h('input', { type: 'password', value: '', placeholder: login ? 'Leave empty to keep the current password' : '', spellcheck: 'false' });
  const note = h('input', { type: 'text', value: login ? login.note : '' });
  const dlg = h('dialog', null, h('h3', { text: login ? 'Edit password' : 'Add password' }),
    h('div', { class: 'fields' }, h('label', null, 'Site', site), h('label', null, 'Username', user), h('label', null, 'Password', pass), h('label', null, 'Note', note)),
    h('div', { class: 'actions' }, h('button', { class: 'btn', text: 'Cancel', onclick: () => dlg.close() }), h('button', { class: 'btn primary', text: 'Save', onclick: () => {
      mizu.request('password_save', { id: login ? login.id : null, origin: site.value.trim(), username: user.value, password: pass.value ? pass.value : (login ? null : ''), note: note.value })
        .then(() => {
          dlg.close();
          showToast('Saved');
          load();
        })
        .catch((e) => showToast(e.message));
    } })));
  document.body.append(dlg);
  dlg.showModal();
  dlg.addEventListener('close', () => dlg.remove());
}

function row(l) {
  const secret = h('span', { class: 'secret', translate: 'no', text: '••••••••••' });
  let shown = false;
  const eye = h('button', { class: 'icon-btn', title: 'Show password' }, icon('eye'));
  eye.addEventListener('click', async () => {
    if (shown) {
      secret.textContent = '••••••••••';
      eye.replaceChildren(icon('eye'));
      eye.title = 'Show password';
      shown = false;
      return;
    }
    try {
      const r = await mizu.request('password_reveal', { id: l.id });
      secret.textContent = r.password;
      eye.replaceChildren(icon('eye-off'));
      eye.title = 'Hide password';
      shown = true;
    } catch (e) {
      showToast(e.message);
    }
  });
  const copyPass = h('button', { class: 'icon-btn', title: 'Copy password' }, icon('copy'));
  copyPass.addEventListener('click', async () => {
    try {
      const r = await mizu.request('password_reveal', { id: l.id });
      await navigator.clipboard.writeText(r.password);
      showToast('Password copied');
    } catch (e) {
      showToast(e.message);
    }
  });
  const copyUser = h('button', { class: 'icon-btn', title: 'Copy username' }, icon('user'));
  copyUser.addEventListener('click', () => navigator.clipboard.writeText(l.username).then(() => showToast('Username copied')));
  const edit = h('button', { class: 'icon-btn', title: 'Edit' }, icon('edit'));
  edit.addEventListener('click', () => editDialog(l));
  const del = h('button', { class: 'icon-btn', title: 'Delete' }, icon('trash'));
  del.addEventListener('click', () => {
    if (confirm(`Delete the saved password for ${l.username || l.host}?`)) mizu.request('password_delete', { id: l.id }).then(load);
  });
  return h('div', { class: 'list-row pw-row' },
    letterAvatar(l.origin, l.host),
    h('a', { class: 'main', href: l.origin, translate: 'no' }, h('div', { class: 't', text: l.host }), h('div', { class: 'u', text: l.username || '—' })),
    secret,
    h('div', { class: 'acts' }, eye, copyPass, copyUser, edit, del));
}

async function load() {
  const r = await mizu.request('passwords_get', { query });
  list.replaceChildren();
  if (!r.logins.length) {
    list.append(h('div', { class: 'empty-state' }, icon('key'), h('div', { text: query ? 'No passwords match.' : 'No saved passwords yet.' }),
      query ? null : h('div', { class: 'muted', text: 'Sign in on any site and Mizu will offer to save the password — or import them from another browser.' })));
  }
  for (const l of r.logins) list.append(row(l));
  list.querySelectorAll('a.main').forEach((a) => a.addEventListener('click', (e) => {
    e.preventDefault();
    mizu.send('open_url', { url: a.getAttribute('href'), disposition: 'new_tab' });
  }));
  neverCard.replaceChildren(h('h2', { text: 'Never saved' }));
  if (!r.never.length) neverCard.append(h('p', { class: 'note', text: 'Sites where you chose "Never for this site" appear here.' }));
  for (const origin of r.never) {
    const rm = h('button', { class: 'icon-btn', title: 'Remove' }, icon('close'));
    rm.addEventListener('click', () => mizu.request('password_never_remove', { origin }).then(load));
    neverCard.append(h('div', { class: 'list-row' }, letterAvatar(origin, origin), h('div', { class: 'main', translate: 'no' }, h('div', { class: 't', text: hostOf(origin) || origin })), h('div', { class: 'acts', style: { opacity: 1 } }, rm)));
  }
  banner.hidden = r.offer_to_save;
}

const add = h('button', { class: 'btn primary', text: 'Add' });
add.addEventListener('click', () => editDialog(null));
const imp = h('button', { class: 'btn' }, icon('import'), h('span', { text: 'Import…' }));
imp.addEventListener('click', () => mizu.request('passwords_import').then((r) => {
  if (r) {
    showToast(`${r.imported} passwords imported`);
    load();
  }
}).catch((e) => showToast(e.message)));
const exp = h('button', { class: 'btn' }, icon('export'), h('span', { text: 'Export…' }));
exp.addEventListener('click', () => {
  if (!confirm('The exported file contains your passwords in plain text. Anyone who opens it can read them. Continue?')) return;
  mizu.request('passwords_export').then((r) => r && r.path && showToast(`${r.count} passwords exported`)).catch((e) => showToast(e.message));
});

const banner = h('div', { class: 'pw-banner', hidden: true }, icon('info'), h('span', { text: 'Offering to save passwords is turned off in Settings → Privacy.' }));
main.append(h('div', { class: 'page' },
  pageHead('Passwords', searchBox('Search passwords', (q) => {
    query = q;
    load();
  }), imp, exp, add),
  banner,
  h('div', { class: 'pw-banner' }, icon('lock'), h('span', { text: 'Passwords are encrypted with your Windows account (DPAPI) and never leave this device. Import accepts CSV exports from Chrome, Edge, Firefox, Opera, Brave, Vivaldi and Bitwarden.' })),
  list,
  neverCard));
bootPage(load);
