// Mizu icon set: 24×24 line icons drawn for Mizu (1.7px stroke, round joins).
// icon('reload') returns an <svg> element; built with DOM APIs, no HTML parsing.
'use strict';

const ICONS = {
  back: [['path', { d: 'M15 5l-7 7 7 7' }]],
  forward: [['path', { d: 'M9 5l7 7-7 7' }]],
  reload: [['path', { d: 'M19.5 12a7.5 7.5 0 1 1-2.2-5.3' }], ['path', { d: 'M19.5 4.5v4h-4' }]],
  stop: [['path', { d: 'M6 6l12 12M18 6L6 18' }]],
  home: [['path', { d: 'M4 11l8-6.5 8 6.5' }], ['path', { d: 'M6.5 9.5V19h11V9.5' }], ['path', { d: 'M10 19v-5h4v5' }]],
  star: [['path', { d: 'M12 3.8l2.5 5.1 5.6.8-4 3.9 1 5.6-5.1-2.7-5 2.7 1-5.6-4.1-3.9 5.6-.8z' }]],
  'star-filled': [['path', { d: 'M12 3.8l2.5 5.1 5.6.8-4 3.9 1 5.6-5.1-2.7-5 2.7 1-5.6-4.1-3.9 5.6-.8z', fill: 'currentColor' }]],
  shield: [['path', { d: 'M12 3.5l7 2.6v5.4c0 4.4-3 7.8-7 9-4-1.2-7-4.6-7-9V6.1z' }]],
  'shield-check': [['path', { d: 'M12 3.5l7 2.6v5.4c0 4.4-3 7.8-7 9-4-1.2-7-4.6-7-9V6.1z' }], ['path', { d: 'M8.8 12.2l2.2 2.2 4.3-4.4' }]],
  'shield-off': [['path', { d: 'M12 3.5l7 2.6v5.4c0 4.4-3 7.8-7 9-4-1.2-7-4.6-7-9V6.1z' }], ['path', { d: 'M4 4l16 16' }]],
  download: [['path', { d: 'M12 4v11' }], ['path', { d: 'M7.5 10.5L12 15l4.5-4.5' }], ['path', { d: 'M5 19.5h14' }]],
  menu: [['circle', { cx: 12, cy: 5.5, r: 1.3, fill: 'currentColor', stroke: 'none' }], ['circle', { cx: 12, cy: 12, r: 1.3, fill: 'currentColor', stroke: 'none' }], ['circle', { cx: 12, cy: 18.5, r: 1.3, fill: 'currentColor', stroke: 'none' }]],
  plus: [['path', { d: 'M12 5v14M5 12h14' }]],
  close: [['path', { d: 'M7 7l10 10M17 7L7 17' }]],
  lock: [['rect', { x: 5.5, y: 10.5, width: 13, height: 9.5, rx: 2 }], ['path', { d: 'M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5' }]],
  warning: [['path', { d: 'M12 4l9 15.5H3z' }], ['path', { d: 'M12 10v4' }], ['circle', { cx: 12, cy: 16.8, r: 0.6, fill: 'currentColor' }]],
  globe: [['circle', { cx: 12, cy: 12, r: 8.5 }], ['path', { d: 'M3.5 12h17M12 3.5c2.4 2.6 3.5 5.5 3.5 8.5s-1.1 5.9-3.5 8.5c-2.4-2.6-3.5-5.5-3.5-8.5s1.1-5.9 3.5-8.5z' }]],
  search: [['circle', { cx: 10.5, cy: 10.5, r: 6 }], ['path', { d: 'M15 15l5 5' }]],
  pin: [['path', { d: 'M9 4h6l-1 5 3 3v2H7v-2l3-3z' }], ['path', { d: 'M12 14v6' }]],
  volume: [['path', { d: 'M4.5 9.5h3l4.5-4v13l-4.5-4h-3z' }], ['path', { d: 'M15.5 9a4.5 4.5 0 0 1 0 6M18 6.5a8 8 0 0 1 0 11' }]],
  mute: [['path', { d: 'M4.5 9.5h3l4.5-4v13l-4.5-4h-3z' }], ['path', { d: 'M16 10l4 4M20 10l-4 4' }]],
  copy: [['rect', { x: 8.5, y: 8.5, width: 11, height: 11, rx: 2 }], ['path', { d: 'M15.5 5.5V5a1.5 1.5 0 0 0-1.5-1.5H6A1.5 1.5 0 0 0 4.5 5v8A1.5 1.5 0 0 0 6 14.5h.5' }]],
  history: [['path', { d: 'M4.5 12a7.5 7.5 0 1 0 2.2-5.3' }], ['path', { d: 'M4.5 4.5v4h4' }], ['path', { d: 'M12 8v4.5l3 2' }]],
  bookmark: [['path', { d: 'M7 4h10v16l-5-3.5L7 20z' }]],
  settings: [['circle', { cx: 12, cy: 12, r: 3 }], ['path', { d: 'M12 3.5l1.6 2.2 2.7-.5.9 2.6 2.6.9-.5 2.7 2.2 1.6-2.2 1.6.5 2.7-2.6.9-.9 2.6-2.7-.5L12 20.5l-1.6-2.2-2.7.5-.9-2.6-2.6-.9.5-2.7L2.5 12l2.2-1.6-.5-2.7 2.6-.9.9-2.6 2.7.5z' }]],
  puzzle: [['path', { d: 'M5 8.5h3.5a2 2 0 1 1 4 0H16V12a2 2 0 1 1 0 4v3.5H5V16a2 2 0 1 0 0-4z' }]],
  code: [['path', { d: 'M9 7l-5 5 5 5M15 7l5 5-5 5' }]],
  info: [['circle', { cx: 12, cy: 12, r: 8.5 }], ['path', { d: 'M12 11v5.5' }], ['circle', { cx: 12, cy: 8, r: 0.6, fill: 'currentColor' }]],
  folder: [['path', { d: 'M3.5 7a1.5 1.5 0 0 1 1.5-1.5h4l2 2h8a1.5 1.5 0 0 1 1.5 1.5v8.5a1.5 1.5 0 0 1-1.5 1.5H5a1.5 1.5 0 0 1-1.5-1.5z' }]],
  file: [['path', { d: 'M6.5 3.5h7l4 4v13h-11z' }], ['path', { d: 'M13.5 3.5v4h4' }]],
  trash: [['path', { d: 'M4.5 7h15M9.5 7V4.5h5V7M6.5 7l1 12.5h9l1-12.5' }]],
  pause: [['path', { d: 'M9 6v12M15 6v12' }]],
  play: [['path', { d: 'M8 5.5l11 6.5-11 6.5z' }]],
  check: [['path', { d: 'M5 12.5l4.5 4.5L19 7.5' }]],
  'chevron-right': [['path', { d: 'M10 6l6 6-6 6' }]],
  'chevron-down': [['path', { d: 'M6 10l6 6 6-6' }]],
  external: [['path', { d: 'M14 4.5h5.5V10M19.5 4.5L11 13' }], ['path', { d: 'M18 14v4.5a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 4 18.5v-11A1.5 1.5 0 0 1 5.5 6H10' }]],
  window: [['rect', { x: 3.5, y: 5, width: 17, height: 14, rx: 2 }], ['path', { d: 'M3.5 9h17' }]],
  private: [['path', { d: 'M3.5 11.5h17' }], ['path', { d: 'M6 11.5l1.5-6h9l1.5 6' }], ['circle', { cx: 8, cy: 16.5, r: 2.5 }], ['circle', { cx: 16, cy: 16.5, r: 2.5 }], ['path', { d: 'M10.5 16.5h3' }]],
  'zoom-in': [['circle', { cx: 10.5, cy: 10.5, r: 6 }], ['path', { d: 'M15 15l5 5M10.5 8v5M8 10.5h5' }]],
  'zoom-out': [['circle', { cx: 10.5, cy: 10.5, r: 6 }], ['path', { d: 'M15 15l5 5M8 10.5h5' }]],
  print: [['path', { d: 'M7 9V4h10v5' }], ['rect', { x: 4, y: 9, width: 16, height: 7.5, rx: 1.5 }], ['path', { d: 'M7.5 14h9v6h-9z' }]],
  devtools: [['rect', { x: 3.5, y: 4.5, width: 17, height: 15, rx: 2 }], ['path', { d: 'M7.5 10l2.5 2-2.5 2M12 15h4' }]],
  minimize: [['path', { d: 'M6 12h12' }]],
  maximize: [['rect', { x: 6, y: 6, width: 12, height: 12, rx: 1.5 }]],
  restore: [['rect', { x: 5.5, y: 8.5, width: 10, height: 10, rx: 1.5 }], ['path', { d: 'M8.5 8.5V6.5A1 1 0 0 1 9.5 5.5h8a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-2' }]],
  fullscreen: [['path', { d: 'M4.5 9V4.5H9M15 4.5h4.5V9M19.5 15v4.5H15M9 19.5H4.5V15' }]],
  clock: [['circle', { cx: 12, cy: 12, r: 8.5 }], ['path', { d: 'M12 7.5V12l3 2' }]],
  drop: [['path', { d: 'M12 3.5c3.2 4 6 7.3 6 10.5a6 6 0 0 1-12 0c0-3.2 2.8-6.5 6-10.5z' }]],
  camera: [['rect', { x: 3.5, y: 6.5, width: 12.5, height: 11, rx: 2 }], ['path', { d: 'M16 10.5l4.5-3v9L16 13.5' }]],
  mic: [['rect', { x: 9, y: 3.5, width: 6, height: 10.5, rx: 3 }], ['path', { d: 'M6 11a6 6 0 0 0 12 0M12 17v3.5' }]],
  location: [['path', { d: 'M12 20.5s6.5-5.6 6.5-10.5a6.5 6.5 0 0 0-13 0c0 4.9 6.5 10.5 6.5 10.5z' }], ['circle', { cx: 12, cy: 10, r: 2.3 }]],
  bell: [['path', { d: 'M6 16.5V11a6 6 0 0 1 12 0v5.5l1.5 1.5h-15z' }], ['path', { d: 'M10 20.5h4' }]],
  clipboard: [['rect', { x: 5.5, y: 5, width: 13, height: 15.5, rx: 2 }], ['path', { d: 'M9 5V3.5h6V5' }]],
  edit: [['path', { d: 'M4.5 19.5l1-4.5L15.5 5a2 2 0 0 1 3 3l-10 10z' }], ['path', { d: 'M13.5 7l3 3' }]],
  sparkle: [['path', { d: 'M12 4l1.8 5.2L19 11l-5.2 1.8L12 18l-1.8-5.2L5 11l5.2-1.8z' }]],
  eye: [['path', { d: 'M2.5 12s3.5-6.5 9.5-6.5S21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z' }], ['circle', { cx: 12, cy: 12, r: 2.8 }]],
  wave: [['path', { d: 'M3 10c2 0 2-2 4.5-2S10 10 12 10s2-2 4.5-2S19 10 21 10M3 15c2 0 2-2 4.5-2S10 15 12 15s2-2 4.5-2S19 15 21 15' }]],
  media: [['rect', { x: 3.5, y: 5.5, width: 17, height: 13, rx: 2 }], ['path', { d: 'M10 9.5l5 2.5-5 2.5z' }]],
  upload: [['path', { d: 'M12 15V4' }], ['path', { d: 'M7.5 8.5L12 4l4.5 4.5' }], ['path', { d: 'M5 19.5h14' }]],
  sun: [['circle', { cx: 12, cy: 12, r: 4 }], ['path', { d: 'M12 2.5v2.5M12 19v2.5M2.5 12H5M19 12h2.5M5.3 5.3l1.8 1.8M16.9 16.9l1.8 1.8M5.3 18.7l1.8-1.8M16.9 7.1l1.8-1.8' }]],
  key: [['circle', { cx: 8, cy: 14, r: 4 }], ['path', { d: 'M11 11l8.5-8.5M16 6l2.5 2.5M14 8l2 2' }]],
  tabs: [['rect', { x: 3.5, y: 6.5, width: 17, height: 13, rx: 2 }], ['path', { d: 'M3.5 10.5h17M8 6.5V4.5h8v2' }]],
  gauge: [['path', { d: 'M4.5 16a7.5 7.5 0 1 1 15 0' }], ['path', { d: 'M12 16l3.5-4.5' }]],
  sliders: [['path', { d: 'M5 7h9M18 7h1M5 17h1M10 17h9' }], ['circle', { cx: 16, cy: 7, r: 2 }], ['circle', { cx: 8, cy: 17, r: 2 }]],
};

function icon(name, opts = {}) {
  const NS = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(NS, 'svg');
  svg.setAttribute('viewBox', '0 0 24 24');
  svg.setAttribute('class', `icon${opts.class ? ' ' + opts.class : ''}`);
  svg.setAttribute('aria-hidden', 'true');
  if (opts.size) {
    svg.style.width = `${opts.size}px`;
    svg.style.height = `${opts.size}px`;
  }
  for (const [tag, attrs] of ICONS[name] || ICONS.globe) {
    const el = document.createElementNS(NS, tag);
    for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, String(v));
    svg.append(el);
  }
  return svg;
}
