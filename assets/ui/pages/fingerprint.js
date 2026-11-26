// Fingerprint check: what websites can read about this PC, measured three times — without
// protection, as "site A" and as "site B" (two blank frames with Mizu's defence and two
// different keys). Nothing is sent anywhere.
'use strict';

const main = document.getElementById('main');

function hash(s) {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 0x01000193) >>> 0;
  return h.toString(16).padStart(8, '0');
}

let strict = false;

function frame(seed) {
  const f = document.createElement('iframe');
  f.style.cssText = 'position:absolute;width:1px;height:1px;opacity:0;pointer-events:none;border:0';
  f.setAttribute('aria-hidden', 'true');
  document.body.append(f);
  if (seed) mizuFingerprintProtect(f.contentWindow, seed, { strict });
  return f.contentWindow;
}

async function measure(win) {
  const out = {};
  try {
    const c = win.document.createElement('canvas');
    c.width = 260;
    c.height = 60;
    const ctx = c.getContext('2d');
    const g = ctx.createLinearGradient(0, 0, 260, 0);
    g.addColorStop(0, '#2f7de1');
    g.addColorStop(1, '#e846a0');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, 260, 60);
    ctx.font = '22px Segoe UI, sans-serif';
    ctx.fillStyle = '#fff';
    ctx.fillText('Mizu 水 fingerprint ✓', 10, 38);
    out.canvas = hash(c.toDataURL());
  } catch {
    out.canvas = '—';
  }
  try {
    const gl = win.document.createElement('canvas').getContext('webgl');
    gl.getExtension('WEBGL_debug_renderer_info');
    out.gpu = String(gl.getParameter(0x9246));
  } catch {
    out.gpu = '—';
  }
  try {
    const ac = new win.OfflineAudioContext(1, 44100, 44100);
    const osc = ac.createOscillator();
    osc.type = 'triangle';
    osc.frequency.value = 10000;
    const comp = ac.createDynamicsCompressor();
    osc.connect(comp);
    comp.connect(ac.destination);
    osc.start(0);
    const buf = await ac.startRendering();
    const data = buf.getChannelData(0);
    let sum = 0;
    for (let i = 4500; i < 5000; i++) sum += Math.abs(data[i]);
    out.audio = sum.toFixed(9);
  } catch {
    out.audio = '—';
  }
  out.screen = `${win.screen.width}×${win.screen.height} · ${win.screen.availWidth}×${win.screen.availHeight}`;
  out.cores = String(win.navigator.hardwareConcurrency ?? '—');
  out.memory = win.navigator.deviceMemory ? `${win.navigator.deviceMemory} GB` : '—';
  out.battery = 'getBattery' in win.navigator ? 'available' : 'hidden';
  out.fonts = 'queryLocalFonts' in win ? 'available' : 'hidden';
  try {
    out.voices = String(win.speechSynthesis.getVoices().length);
  } catch {
    out.voices = '—';
  }
  try {
    const v = await win.navigator.userAgentData.getHighEntropyValues(['platformVersion', 'uaFullVersion']);
    out.platform = `Windows ${v.platformVersion} · ${v.uaFullVersion}`;
  } catch {
    out.platform = '—';
  }
  return out;
}

const ROWS = [
  ['canvas', 'Canvas drawing', 'How your graphics card draws a picture. Mizu adds invisible noise: each site sees a different result.', 'site'],
  ['audio', 'Audio processing', 'How your PC processes sound. Mizu adds inaudible noise per site.', 'site'],
  ['gpu', 'Graphics card', 'Sites see only the maker, not the model or driver. Strict: a generic card for everyone.', 'same'],
  ['screen', 'Screen size', 'Screen and usable area (without the taskbar). Strict: a common size and no taskbar.', 'same'],
  ['cores', 'Processor cores', 'Rounded to a common value.', 'same'],
  ['memory', 'Memory', 'Rounded to a common value.', 'same'],
  ['battery', 'Battery level', 'Hidden: ordinary sites do not need it.', 'same'],
  ['fonts', 'Installed fonts list', 'Hidden: sites cannot list your fonts.', 'same'],
  ['voices', 'Speech voices', 'Only the ones for your language and English.', 'same'],
  ['platform', 'Windows and browser build', 'Only the major versions.', 'same'],
];

async function run(settings) {
  const on = settings.privacy.fingerprinting !== 'off';
  strict = settings.privacy.fingerprinting === 'strict';
  const body = h('div', { class: 'card fp-card' }, h('div', { class: 'fp-loading', text: 'Measuring…' }));
  const again = h('button', { class: 'btn' }, icon('reload'), h('span', { text: 'Measure again' }));
  again.addEventListener('click', () => location.reload());
  const toSettings = h('button', { class: 'btn', text: 'Privacy settings' });
  toSettings.addEventListener('click', () => mizu.send('open_url', { url: 'mizu://settings/#privacy', disposition: 'current' }));
  main.replaceChildren(h('div', { class: 'page fp' },
    pageHead('Fingerprint check', again, toSettings),
    h('div', { class: `fp-banner ${on ? 'on' : 'off'}` }, icon(on ? 'shield-check' : 'shield-off', { size: 20 }), h('div', null,
      h('b', { text: on ? (strict ? 'Fingerprinting defence is on (strict)' : 'Fingerprinting defence is on') : 'Fingerprinting defence is off' }),
      h('span', { text: on ? 'Websites see the values on the right, different for every site and new each time Mizu starts.' : 'Websites see the real values. The columns on the right show what they would see with the defence on.' }))),
    body,
    h('p', { class: 'muted', text: 'Measured on this page only; nothing is sent anywhere. Fingerprinting defence makes tracking harder, not impossible: sites still see your IP address and recognise you when you sign in.' })));

  const [real, a, b] = await Promise.all([measure(window), measure(frame(0x5a17e1)), measure(frame(0x0b1e55))]);
  const table = h('div', { class: 'fp-table' },
    h('div', { class: 'fp-row head' }, h('span'), h('span', { text: 'Without protection' }), h('span', { text: 'Site A' }), h('span', { text: 'Site B' })));
  for (const [k, title, note, kind] of ROWS) {
    const changed = a[k] !== real[k];
    const perSite = a[k] !== b[k];
    const badge = kind === 'site' ? (perSite ? ['ok', 'Different on every site'] : ['', 'Unchanged']) : changed ? ['ok', 'Protected'] : ['', 'Already common'];
    table.append(h('div', { class: 'fp-row' },
      h('span', { class: 'fp-name' }, h('b', { text: title }), h('small', { text: note }), h('i', { class: `pill ${badge[0]}`, text: badge[1] })),
      h('code', { translate: 'no', text: real[k] }),
      h('code', { class: changed ? 'diff' : '', translate: 'no', text: a[k] }),
      h('code', { class: b[k] !== real[k] ? 'diff' : '', translate: 'no', text: b[k] })));
  }
  body.replaceChildren(table);
}

bootPage(() => mizu.request('get_settings').then((p) => run(p.settings)));
