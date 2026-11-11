// Bookmarks manager: folder tree, editing, drag-and-drop ordering, search.
'use strict';

const main = document.getElementById('main');
let all = [];
let folder = 1; // Bookmarks bar
let query = '';

const tree = h('nav', { class: 'bm-tree' });
const list = h('section', { class: 'card bm-list' });

const childrenOf = (id) => all.filter((b) => b.parent_id === id).sort((a, b) => a.position - b.position);
const byId = (id) => all.find((b) => b.id === id);

async function reload() {
  all = await mizu.request('bookmarks_get');
  if (!byId(folder)) folder = 1;
  render();
}

function editDialog(b) {
  const isFolder = b.kind === 'folder';
  const title = h('input', { type: 'text', value: b.title || '' });
  const url = h('input', { type: 'url', value: b.url || '' });
  const dlg = h('dialog', null, h('h3', { text: b.id ? (isFolder ? 'Rename folder' : 'Edit bookmark') : isFolder ? 'New folder' : 'Add bookmark' }),
    h('div', { class: 'fields' }, h('label', null, 'Name', title), isFolder ? null : h('label', null, 'URL', url)),
    h('div', { class: 'actions' }, h('button', { class: 'btn', text: 'Cancel', onclick: () => dlg.close() }), h('button', { class: 'btn primary', text: 'Save', onclick: async () => {
      try {
        const name = title.value.trim() || (isFolder ? 'New folder' : url.value.trim());
        if (b.id) await mizu.request('bookmark_update', { id: b.id, title: name, url: isFolder ? null : url.value.trim() });
        else if (isFolder) await mizu.request('bookmark_add_folder', { parent: folder, title: name });
        else {
          if (!/^(https?|file|mizu):/i.test(url.value.trim())) throw new Error('Enter a full address, e.g. https://example.com');
          await mizu.request('bookmark_add', { parent: folder, title: name, url: url.value.trim() });
        }
        dlg.close();
        reload();
      } catch (e) {
        showToast(e.message);
      }
    } })));
  document.body.append(dlg);
  dlg.showModal();
  dlg.addEventListener('close', () => dlg.remove());
}

function renderTree() {
  tree.replaceChildren();
  const walk = (parentId, depth) => {
    for (const f of childrenOf(parentId).filter((b) => b.kind === 'folder')) {
      const btn = h('button', { class: f.id === folder && !query ? 'on' : '', style: { paddingLeft: `${12 + depth * 16}px` } }, icon('folder'), h('span', { text: f.title }));
      btn.addEventListener('click', () => {
        folder = f.id;
        query = '';
        searchInput.value = '';
        render();
      });
      // Drop bookmarks onto folders.
      btn.addEventListener('dragover', (e) => e.preventDefault());
      btn.addEventListener('drop', (e) => {
        e.preventDefault();
        const id = Number(e.dataTransfer.getData('text/mizu-bookmark'));
        if (id && id !== f.id) mizu.request('bookmark_move', { id, parent: f.id, index: 1e6 }).then(reload).catch((err) => showToast(err.message));
      });
      tree.append(btn);
      walk(f.id, depth + 1);
    }
  };
  for (const root of all.filter((b) => b.parent_id === null)) {
    const btn = h('button', { class: root.id === folder && !query ? 'on' : '' }, icon(root.id === 1 ? 'bookmark' : 'folder'), h('span', { text: root.title }));
    btn.addEventListener('click', () => {
      folder = root.id;
      query = '';
      searchInput.value = '';
      render();
    });
    btn.addEventListener('dragover', (e) => e.preventDefault());
    btn.addEventListener('drop', (e) => {
      e.preventDefault();
      const id = Number(e.dataTransfer.getData('text/mizu-bookmark'));
      if (id) mizu.request('bookmark_move', { id, parent: root.id, index: 1e6 }).then(reload);
    });
    tree.append(btn);
    walk(root.id, 1);
  }
}

function itemRow(b, index) {
  const isFolder = b.kind === 'folder';
  const edit = h('button', { class: 'icon-btn', title: 'Edit' }, icon('edit'));
  const del = h('button', { class: 'icon-btn', title: 'Delete' }, icon('trash'));
  const r = h('div', { class: 'list-row', draggable: 'true' },
    isFolder ? h('div', { class: 'fav' }, icon('folder')) : letterAvatar(b.url, b.title),
    h('div', { class: 'main', translate: 'no' }, h('div', { class: 't', text: b.title }), isFolder ? h('div', { class: 'u', text: `${childrenOf(b.id).length} items` }) : h('div', { class: 'u', text: b.url })),
    h('div', { class: 'acts' }, edit, del));
  r.addEventListener('dblclick', () => {
    if (isFolder) {
      folder = b.id;
      render();
    } else mizu.send('open_url', { url: b.url, disposition: 'current' });
  });
  r.querySelector('.main').addEventListener('click', (e) => {
    if (isFolder) {
      folder = b.id;
      render();
    } else mizu.send('open_url', { url: b.url, disposition: e.ctrlKey ? 'background' : 'current' });
  });
  edit.addEventListener('click', () => editDialog(b));
  del.addEventListener('click', () => {
    if (isFolder && childrenOf(b.id).length && !confirm(`Delete "${b.title}" and everything in it?`)) return;
    mizu.request('bookmark_delete', { id: b.id }).then(reload);
  });
  r.addEventListener('dragstart', (e) => {
    e.dataTransfer.setData('text/mizu-bookmark', String(b.id));
    r.classList.add('drag');
  });
  r.addEventListener('dragend', () => r.classList.remove('drag'));
  r.addEventListener('dragover', (e) => {
    e.preventDefault();
    r.classList.add('drop-target');
  });
  r.addEventListener('dragleave', () => r.classList.remove('drop-target'));
  r.addEventListener('drop', (e) => {
    e.preventDefault();
    r.classList.remove('drop-target');
    const id = Number(e.dataTransfer.getData('text/mizu-bookmark'));
    if (id && id !== b.id && !query) mizu.request('bookmark_move', { id, parent: b.parent_id, index }).then(reload).catch((err) => showToast(err.message));
  });
  return r;
}

function renderList() {
  list.replaceChildren();
  let items;
  if (query) {
    const q = query.toLowerCase();
    items = all.filter((b) => b.kind === 'url' && ((b.title || '').toLowerCase().includes(q) || (b.url || '').toLowerCase().includes(q)));
  } else {
    items = childrenOf(folder);
    const path = [];
    for (let f = byId(folder); f; f = f.parent_id ? byId(f.parent_id) : null) path.unshift(f.title);
    list.append(h('div', { class: 'group-label', text: path.join(' › ') }));
  }
  items.forEach((b, i) => list.append(itemRow(b, i)));
  if (!items.length) list.append(h('div', { class: 'empty-state' }, icon('bookmark'), h('div', { text: query ? 'No bookmarks match.' : 'This folder is empty. Press Ctrl+D on any page to bookmark it.' })));
}

function render() {
  renderTree();
  renderList();
}

const searchWrap = searchBox('Search bookmarks', (q) => {
  query = q;
  render();
});
const searchInput = searchWrap.querySelector('input');
const addBtn = h('button', { class: 'btn', text: 'Add bookmark' });
addBtn.addEventListener('click', () => editDialog({ kind: 'url' }));
const folderBtn = h('button', { class: 'btn', text: 'New folder' });
folderBtn.addEventListener('click', () => editDialog({ kind: 'folder' }));

main.append(h('div', { class: 'page', style: { maxWidth: '1080px' } }, pageHead('Bookmarks', searchWrap, folderBtn, addBtn), h('div', { class: 'bm-layout' }, tree, list)));
bootPage(reload);
