// Messaging with the browser process. Loaded by every Mizu surface and internal page.
//
//   mizu.send('new_tab')                       fire-and-forget command
//   await mizu.request('history_query', {...}) command with a reply
//   mizu.on('state', (msg) => …)               push events from the browser
'use strict';

const mizu = (() => {
  const webview = window.chrome && window.chrome.webview;
  let nextId = 1;
  const pending = new Map();
  const handlers = new Map();

  if (webview) {
    webview.addEventListener('message', (event) => {
      const msg = event.data;
      if (!msg || typeof msg !== 'object') return;
      if (msg.type === 'reply') {
        const p = pending.get(msg.rid);
        if (!p) return;
        pending.delete(msg.rid);
        if (msg.ok) p.resolve(msg.data === undefined ? null : msg.data);
        else p.reject(new Error(msg.error || 'Request failed'));
        return;
      }
      for (const fn of handlers.get(msg.type) || []) {
        try {
          fn(msg);
        } catch (e) {
          console.error('[mizu]', msg.type, e);
        }
      }
    });
  }

  return {
    available: !!webview,
    send(cmd, args = {}) {
      if (webview) webview.postMessage({ ...args, cmd });
    },
    request(cmd, args = {}) {
      if (!webview) return Promise.reject(new Error('Not running inside Mizu'));
      const rid = nextId++;
      return new Promise((resolve, reject) => {
        pending.set(rid, { resolve, reject });
        webview.postMessage({ ...args, rid, cmd });
        setTimeout(() => {
          if (pending.delete(rid)) reject(new Error('Timed out'));
        }, 120000);
      });
    },
    on(type, fn) {
      if (!handlers.has(type)) handlers.set(type, []);
      handlers.get(type).push(fn);
    },
  };
})();

// Small DOM helper used by all surfaces: never parses HTML, so data from web pages (titles,
// URLs) can never become markup.
function h(tag, props, ...children) {
  const el = document.createElement(tag);
  if (props) {
    for (const [k, v] of Object.entries(props)) {
      if (v === undefined || v === null || v === false) continue;
      if (k === 'class') el.className = v;
      else if (k === 'text') el.textContent = v;
      else if (k === 'style' && typeof v === 'object') Object.assign(el.style, v);
      else if (k.startsWith('on') && typeof v === 'function') el.addEventListener(k.slice(2), v);
      else if (k === 'dataset') Object.assign(el.dataset, v);
      else if (v === true) el.setAttribute(k, '');
      else el.setAttribute(k, String(v));
    }
  }
  for (const c of children.flat()) {
    if (c === null || c === undefined || c === false) continue;
    el.append(c instanceof Node ? c : document.createTextNode(String(c)));
  }
  return el;
}

function formatBytes(n) {
  if (!(n >= 0)) return '';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let i = 0;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i++;
  }
  return `${n < 10 && i > 0 ? n.toFixed(1) : Math.round(n)} ${units[i]}`;
}

function formatDuration(sec) {
  if (!(sec >= 0)) return '';
  if (sec < 60) return `${Math.max(1, Math.round(sec))} s`;
  if (sec < 3600) return `${Math.round(sec / 60)} min`;
  return `${Math.floor(sec / 3600)} h ${Math.round((sec % 3600) / 60)} min`;
}

function hostOf(url) {
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return '';
  }
}

// ── Helpers for internal pages ──────────────────────────────
let toastTimer = 0;
function showToast(message) {
  const el = document.getElementById('toast');
  if (!el) return;
  el.textContent = message;
  el.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.remove('show'), 2800);
}

function pageHead(title, ...extra) {
  return h('header', { class: 'page-head' }, h('img', { src: '/_/logo.svg', alt: '' }), h('h1', { text: title }), h('div', { class: 'grow' }), ...extra);
}

function searchBox(placeholder, onInput) {
  const input = h('input', { type: 'search', placeholder });
  let t = 0;
  input.addEventListener('input', () => {
    clearTimeout(t);
    t = setTimeout(() => onInput(input.value), 160);
  });
  return h('div', { class: 'search' }, icon('search'), input);
}

function switchInput(checked, onChange, opts = {}) {
  const el = h('input', { type: 'checkbox', class: 'switch', checked: !!checked, disabled: !!opts.disabled, 'aria-label': opts.label || '' });
  el.addEventListener('change', () => onChange(el.checked));
  return el;
}

function relativeTime(ms) {
  const d = new Date(ms);
  const now = new Date();
  const sameDay = d.toDateString() === now.toDateString();
  if (sameDay) return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  return d.toLocaleDateString([], { day: 'numeric', month: 'short', year: d.getFullYear() === now.getFullYear() ? undefined : 'numeric' });
}

function letterAvatar(url, title) {
  const s = hostOf(url) || title || '?';
  let hash = 0;
  for (const c of s) hash = (hash * 31 + c.charCodeAt(0)) | 0;
  const hue = Math.abs(hash) % 360;
  return h('div', { class: 'fav', text: s.charAt(0).toUpperCase(), style: { background: `hsl(${hue} 55% 55% / 0.18)`, color: `hsl(${hue} 50% 45%)` } });
}
