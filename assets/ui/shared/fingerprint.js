// Mizu fingerprinting defence. Standard: gives high-entropy APIs protective values — a little
// noise in canvas, WebGL and audio readback, rounded hardware numbers, no battery or local
// fonts API, fewer devices and voices before permission, and reduced user-agent hints.
// Strict also gives every site the same generic graphics card, a common screen size with no
// taskbar or window position, and fixed hardware numbers.
// Noise depends on a key per site and session: one site always sees the same values during a
// session, two sites see different ones, and nothing is stable across restarts.
// With `report`, every script that reads these APIs is noted (script address and kind of
// API, nothing else) and sent to Mizu for the Shield panel; frames pass theirs to the top page.
// Mizu injects this into web pages (`mizuFingerprintProtect(window, key, {strict, report})`);
// the check page mizu://fingerprint applies it to blank frames to show the difference.
'use strict';

function mizuFingerprintProtect(win, seed, opts) {
  if (!win || win.__mizuFp) return;
  try {
    Object.defineProperty(win, '__mizuFp', { value: true });
  } catch {
    return;
  }
  const key = (Number(seed) >>> 0) || 1;
  const strict = !!(opts && opts.strict);
  const note = opts && opts.report ? mizuFingerprintReporter(win) : () => {};
  // A small, fast hash: the same inputs always give the same noise.
  const mix = (a, b) => {
    let h = (a ^ Math.imul(b | 0, 0x9e3779b1)) >>> 0;
    h = Math.imul(h ^ (h >>> 16), 0x85ebca6b) >>> 0;
    h = Math.imul(h ^ (h >>> 13), 0xc2b2ae35) >>> 0;
    return (h ^ (h >>> 16)) >>> 0;
  };

  // Patched functions still look built in.
  const natives = new WeakMap();
  const FnProto = win.Function.prototype;
  const realToString = FnProto.toString;
  const toString = function toString() {
    return natives.has(this) ? natives.get(this) : realToString.call(this);
  };
  natives.set(toString, realToString.call(realToString));
  try {
    Object.defineProperty(FnProto, 'toString', { value: toString, writable: true, configurable: true });
  } catch {}
  const wrap = (obj, name, make) => {
    if (!obj) return;
    const orig = obj[name];
    if (typeof orig !== 'function') return;
    const fn = make(orig);
    natives.set(fn, realToString.call(orig));
    try {
      Object.defineProperty(fn, 'name', { value: name });
      Object.defineProperty(obj, name, { value: fn, writable: true, configurable: true, enumerable: false });
    } catch {}
  };
  const getter = (obj, name, value) => {
    if (!obj || !(name in obj)) return;
    const desc = Object.getOwnPropertyDescriptor(obj, name);
    const get = function () {
      return typeof value === 'function' ? value.call(this, desc && desc.get) : value;
    };
    if (desc && desc.get) natives.set(get, realToString.call(desc.get));
    try {
      Object.defineProperty(obj, name, { get, configurable: true, enumerable: desc ? desc.enumerable : true });
    } catch {}
  };

  // ── Canvas: about 1 in 120 visible pixels changes by one step in one colour ──
  const noisePixels = (data, width, x0, y0) => {
    for (let i = 0; i < data.length; i += 4) {
      if (data[i + 3] === 0) continue;
      const p = i >> 2;
      const x = x0 + (p % width);
      const y = y0 + ((p / width) | 0);
      const r = mix(key, x * 73856093 ^ y * 19349663);
      if (r % 120 !== 0) continue;
      const c = i + (r >>> 8) % 3;
      data[c] = data[c] === 255 ? 254 : data[c] + 1;
    }
  };
  const C2D = win.CanvasRenderingContext2D && win.CanvasRenderingContext2D.prototype;
  const realGetImageData = C2D && C2D.getImageData;
  wrap(C2D, 'getImageData', (orig) => function (sx, sy, sw, sh, ...rest) {
    note('canvas');
    const img = orig.call(this, sx, sy, sw, sh, ...rest);
    try {
      noisePixels(img.data, img.width, Math.floor(sx) || 0, Math.floor(sy) || 0);
    } catch {}
    return img;
  });
  // toDataURL / toBlob read a copy with the same noise.
  const noisyCopy = (canvas) => {
    const w = canvas.width;
    const hgt = canvas.height;
    if (!w || !hgt || w * hgt > 16_000_000 || !realGetImageData) return null;
    const copy = win.document.createElement('canvas');
    copy.width = w;
    copy.height = hgt;
    const ctx = copy.getContext('2d');
    if (!ctx) return null;
    ctx.drawImage(canvas, 0, 0);
    const img = realGetImageData.call(ctx, 0, 0, w, hgt);
    noisePixels(img.data, w, 0, 0);
    ctx.putImageData(img, 0, 0);
    return copy;
  };
  const Canvas = win.HTMLCanvasElement && win.HTMLCanvasElement.prototype;
  wrap(Canvas, 'toDataURL', (orig) => function (...args) {
    note('canvas');
    let copy = null;
    try {
      copy = noisyCopy(this);
    } catch {}
    return orig.apply(copy || this, args);
  });
  wrap(Canvas, 'toBlob', (orig) => function (...args) {
    note('canvas');
    let copy = null;
    try {
      copy = noisyCopy(this);
    } catch {}
    return orig.apply(copy || this, args);
  });

  // ── WebGL: the GPU's family only (strict: nothing), and noisy readPixels ──
  const DEBUG_INFO = 'WEBGL_debug_renderer_info';
  const generic = (s) => {
    if (strict) return 'ANGLE (Generic, Generic GPU, Direct3D11)';
    const m = /ANGLE \(([^,]+),/.exec(String(s));
    return m ? `ANGLE (${m[1].trim()}, ${m[1].trim()} GPU, Direct3D11)` : String(s).split(/[ (]/)[0] || 'GPU';
  };
  for (const name of ['WebGLRenderingContext', 'WebGL2RenderingContext']) {
    const P = win[name] && win[name].prototype;
    wrap(P, 'getParameter', (orig) => function (p) {
      const v = orig.call(this, p);
      if (p === 0x9246) {
        note('webgl');
        return generic(v);
      }
      if (p === 0x9245) {
        note('webgl');
        return strict ? 'Google Inc.' : v;
      }
      return v;
    });
    wrap(P, 'getExtension', (orig) => function (name) {
      if (name === DEBUG_INFO) {
        note('webgl');
        if (strict) return null;
      }
      return orig.call(this, name);
    });
    if (strict) {
      wrap(P, 'getSupportedExtensions', (orig) => function (...args) {
        const list = orig.apply(this, args);
        return Array.isArray(list) ? list.filter((e) => e !== DEBUG_INFO) : list;
      });
    }
    wrap(P, 'readPixels', (orig) => function (...args) {
      note('webgl');
      const r = orig.apply(this, args);
      try {
        const out = args[6];
        if (out && out.length) {
          for (let i = 0; i < out.length; i += 4) {
            if (mix(key, i) % 151 === 0) out[i] = out[i] === 255 ? 254 : out[i] + 1;
          }
        }
      } catch {}
      return r;
    });
  }

  // ── Audio: inaudible noise in rendered samples and analyser output ──
  const touched = new WeakSet();
  wrap(win.AudioBuffer && win.AudioBuffer.prototype, 'getChannelData', (orig) => function (...args) {
    note('audio');
    const data = orig.apply(this, args);
    try {
      if (!touched.has(data)) {
        touched.add(data);
        for (let i = 0; i < data.length; i++) {
          const r = mix(key, i);
          if (r % 97 === 0) data[i] += ((r >>> 8) / 0xffffff - 0.5) * 2e-7;
        }
      }
    } catch {}
    return data;
  });
  wrap(win.AnalyserNode && win.AnalyserNode.prototype, 'getFloatFrequencyData', (orig) => function (arr) {
    note('audio');
    orig.call(this, arr);
    try {
      for (let i = 0; i < arr.length; i++) arr[i] += ((mix(key, i) >>> 8) / 0xffffff - 0.5) * 1e-4;
    } catch {}
  });

  // ── Hardware: common values only (strict: the same for everyone) ──
  const Nav = win.Navigator && win.Navigator.prototype;
  getter(Nav, 'hardwareConcurrency', (real) => {
    note('hardware');
    if (strict) return 4;
    const n = real ? real.call(win.navigator) : 4;
    return n >= 8 ? 8 : 4;
  });
  getter(Nav, 'deviceMemory', (real) => {
    note('hardware');
    if (strict) return 8;
    const n = real ? real.call(win.navigator) : 8;
    return n >= 8 ? 8 : 4;
  });

  // ── Screen: reads are noted; strict gives a common size, no taskbar, no window position ──
  const SIZES = [[1280, 720], [1366, 768], [1536, 864], [1600, 900], [1920, 1080], [2560, 1440], [3840, 2160]];
  const SP = win.Screen && win.Screen.prototype;
  const realScreen = {};
  if (SP) for (const n of ['width', 'height']) realScreen[n] = Object.getOwnPropertyDescriptor(SP, n);
  // The largest common size that fits the real screen (the same in every frame of a page).
  const sized = (self) => {
    const read = (n) => (realScreen[n] && realScreen[n].get ? realScreen[n].get.call(self) : 0);
    const w = read('width');
    const hh = read('height');
    let pick = SIZES[0];
    for (const z of SIZES) if (z[0] <= w && z[1] <= hh) pick = z;
    return pick;
  };
  const screenGetter = (name, value) => getter(SP, name, function (real) {
    note('screen');
    return strict ? value(this) : real ? real.call(this) : undefined;
  });
  screenGetter('width', (self) => sized(self)[0]);
  screenGetter('height', (self) => sized(self)[1]);
  screenGetter('availWidth', (self) => sized(self)[0]);
  screenGetter('availHeight', (self) => sized(self)[1]);
  screenGetter('availLeft', () => 0);
  screenGetter('availTop', () => 0);
  screenGetter('colorDepth', () => 24);
  screenGetter('pixelDepth', () => 24);
  if (strict) {
    // Window size with borders and position on screen give away the taskbar and scaling.
    const target = (name) => (Object.getOwnPropertyDescriptor(win, name) ? win : win.Window && win.Window.prototype);
    getter(target('outerWidth'), 'outerWidth', () => win.innerWidth);
    getter(target('outerHeight'), 'outerHeight', () => win.innerHeight);
    for (const n of ['screenX', 'screenY', 'screenLeft', 'screenTop']) getter(target(n), n, () => 0);
  }

  // ── APIs ordinary sites do not need ──
  try {
    delete Nav.getBattery;
  } catch {}
  try {
    if ('queryLocalFonts' in win) delete win.queryLocalFonts;
    if (win.Window && 'queryLocalFonts' in win.Window.prototype) delete win.Window.prototype.queryLocalFonts;
  } catch {}

  // ── Cameras and microphones: one of each kind until you allow access ──
  const MD = win.MediaDevices && win.MediaDevices.prototype;
  wrap(MD, 'enumerateDevices', (orig) => function (...args) {
    note('devices');
    return orig.apply(this, args).then((list) => {
      if (list.some((d) => d.label)) return list;
      const seen = new Set();
      return list.filter((d) => (seen.has(d.kind) ? false : seen.add(d.kind)));
    });
  });

  // ── Voices: the ones for your language and English ──
  wrap(win.SpeechSynthesis && win.SpeechSynthesis.prototype, 'getVoices', (orig) => function (...args) {
    note('voices');
    const all = orig.apply(this, args);
    const lang = String(win.navigator.language || 'en').slice(0, 2).toLowerCase();
    return all.filter((v) => v.default || /^en/i.test(v.lang) || v.lang.toLowerCase().startsWith(lang)).slice(0, 4);
  });

  // ── User-agent hints: no exact Windows build, browser build or device model ──
  wrap(win.NavigatorUAData && win.NavigatorUAData.prototype, 'getHighEntropyValues', (orig) => function (hints) {
    note('ua');
    return orig.call(this, hints).then((v) => {
      const major = (s) => `${String(s || '').split('.')[0]}.0.0.0`;
      const out = { ...v };
      if ('platformVersion' in out) out.platformVersion = '10.0.0';
      if ('uaFullVersion' in out) out.uaFullVersion = major(out.uaFullVersion);
      if (Array.isArray(out.fullVersionList)) out.fullVersionList = out.fullVersionList.map((b) => ({ brand: b.brand, version: major(b.version) }));
      if ('model' in out) out.model = '';
      if ('wow64' in out) out.wow64 = false;
      return out;
    });
  });

  // ── Fonts: asking whether a font is installed is noted ──
  wrap(win.FontFaceSet && win.FontFaceSet.prototype, 'check', (orig) => function (...args) {
    note('fonts');
    return orig.apply(this, args);
  });
}

// Notes which scripts read fingerprintable APIs. Each kind of API is attributed for its first
// calls only (reading the call stack is not free); new script/API pairs go to Mizu in batches.
function mizuFingerprintReporter(win) {
  const URL_RE = /(https?:\/\/[^\s()]+?):\d+:\d+/g;
  let self = '';
  const scriptOf = (stack) => {
    for (const m of String(stack || '').matchAll(URL_RE)) {
      const u = m[1].split(/[?#]/)[0];
      if (u && u !== self) return u.slice(0, 200);
    }
    return '';
  };
  try {
    self = scriptOf(new win.Error().stack);
  } catch {}
  const top = win === win.top;
  const wv = top && win.chrome && win.chrome.webview;
  const post = wv ? wv.postMessage.bind(wv) : null;
  const send = (items) => {
    try {
      if (post) post({ mizu_fp: { items } });
      else win.top.postMessage({ __mizuFpReport: items }, '*');
    } catch {}
  };
  if (post) {
    win.addEventListener('message', (e) => {
      const r = e.data && e.data.__mizuFpReport;
      if (Array.isArray(r) && e.source !== win) send(r.slice(0, 100));
    });
  }
  const calls = Object.create(null);
  const seen = new Set();
  let queue = [];
  let timer = 0;
  const later = win.setTimeout.bind(win);
  const flush = () => {
    timer = 0;
    if (queue.length) send(queue);
    queue = [];
  };
  return (api) => {
    const n = (calls[api] = (calls[api] || 0) + 1);
    if (n > 25 || seen.size > 300) return;
    let script = '';
    try {
      script = scriptOf(new win.Error().stack);
    } catch {}
    const id = `${script} ${api}`;
    if (seen.has(id)) return;
    seen.add(id);
    queue.push([script, api]);
    if (!timer) timer = later(flush, 400);
  };
}
