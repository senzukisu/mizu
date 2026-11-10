// Media diagnostics: what the engine can play (probed live) and which system decoders exist.
'use strict';

const main = document.getElementById('main');

const VIDEO = [
  ['H.264 / AVC', 'video/mp4; codecs="avc1.640028"', 'H.264 / AVC'],
  ['H.265 / HEVC', 'video/mp4; codecs="hvc1.1.6.L120.90"', 'H.265 / HEVC'],
  ['AV1', 'video/mp4; codecs="av01.0.08M.08"', 'AV1'],
  ['VP9', 'video/webm; codecs="vp09.00.40.08"', 'VP9'],
  ['VP8', 'video/webm; codecs="vp8"', 'VP8'],
  ['MPEG-4 Part 2', 'video/mp4; codecs="mp4v.20.8"', 'MPEG-4 Part 2'],
  ['MPEG-2', 'video/mp2t; codecs="mp2v"', 'MPEG-2'],
  ['Theora', 'video/ogg; codecs="theora"', 'Theora'],
];
const AUDIO = [
  ['AAC', 'audio/mp4; codecs="mp4a.40.2"', 'AAC'],
  ['MP3', 'audio/mpeg', 'MP3'],
  ['Opus', 'audio/webm; codecs="opus"', 'Opus'],
  ['Vorbis', 'audio/webm; codecs="vorbis"', null],
  ['FLAC', 'audio/flac', 'FLAC'],
  ['PCM (WAV)', 'audio/wav; codecs="1"', null],
  ['ALAC', 'audio/mp4; codecs="alac"', 'ALAC'],
  ['AC-3', 'audio/mp4; codecs="ac-3"', 'AC-3'],
  ['E-AC-3', 'audio/mp4; codecs="ec-3"', 'E-AC-3'],
];
const CONTAINERS = [
  ['MP4', 'video/mp4'],
  ['WebM', 'video/webm'],
  ['Matroska (MKV)', 'video/x-matroska'],
  ['QuickTime (MOV)', 'video/quicktime'],
  ['AVI', 'video/x-msvideo'],
  ['Ogg', 'video/ogg'],
  ['MPEG-TS', 'video/mp2t'],
  ['WAV', 'audio/wav'],
  ['FLAC', 'audio/flac'],
];

const probeEl = document.createElement('video');
const canPlay = (type) => probeEl.canPlayType(type); // "", "maybe", "probably"
const mse = (type) => (window.MediaSource && MediaSource.isTypeSupported(type)) || false;

async function capabilities(type, kind) {
  if (!navigator.mediaCapabilities) return null;
  const cfg = kind === 'video'
    ? { type: 'media-source', video: { contentType: type, width: 1920, height: 1080, bitrate: 6e6, framerate: 30 } }
    : { type: 'media-source', audio: { contentType: type, channels: '2', bitrate: 160000, samplerate: 48000 } };
  try {
    return await navigator.mediaCapabilities.decodingInfo(cfg);
  } catch {
    return null;
  }
}

function yes(v, label) {
  return h('span', { class: `pill ${v ? 'ok' : ''}`, text: v ? label || 'Yes' : 'No' });
}

async function codecTable(rows, kind, system) {
  const table = h('table', { class: 'grid' }, h('tr', null, h('th', { text: 'Codec' }), h('th', { text: 'Files' }), h('th', { text: 'Streaming (MSE)' }), h('th', { text: kind === 'video' ? 'Smooth 1080p / hardware' : 'Smooth' }), h('th', { text: 'System decoders' })));
  for (const [name, type, sysKey] of rows) {
    const cp = canPlay(type);
    const ms = mse(type);
    const caps = await capabilities(type, kind);
    const sys = system && sysKey ? system.codecs.find((c) => c.codec === sysKey) : null;
    const sysText = sys ? [sys.hardware.length ? `HW: ${sys.hardware.join(', ')}` : null, sys.software.length ? `SW: ${sys.software.join(', ')}` : null].filter(Boolean).join(' · ') || 'none' : '—';
    const perf = caps && caps.supported ? h('span', { class: `pill ${caps.powerEfficient ? 'accent' : caps.smooth ? 'ok' : ''}`, text: caps.powerEfficient ? 'Hardware' : caps.smooth ? 'Software' : 'Slow' }) : h('span', { class: 'pill', text: '—' });
    table.append(h('tr', null, h('td', { text: name }), h('td', null, yes(cp !== '', cp === 'probably' ? 'Yes' : 'Likely')), h('td', null, yes(ms)), h('td', null, perf), h('td', { class: 'muted', text: sysText })));
  }
  return table;
}

async function render() {
  const status = h('p', { class: 'note', text: 'Probing system decoders…' });
  main.replaceChildren(h('div', { class: 'page', style: { maxWidth: '1100px' } }, pageHead('Media diagnostics'), status));
  let system = null;
  try {
    system = await mizu.request('media_probe');
  } catch (e) {
    status.textContent = `System probe failed: ${e.message}`;
  }
  const page = h('div', { class: 'page', style: { maxWidth: '1100px' } }, pageHead('Media diagnostics'));
  page.append(h('p', { class: 'muted', text: 'Playback is handled by the Chromium media stack in the engine. "Files" and "Streaming" are what websites can actually use; "System decoders" are Windows Media Foundation decoders the engine can use for hardware acceleration (and for HEVC, which needs a GPU decoder plus the HEVC Video Extensions from the Microsoft Store). Mizu does not bundle any codecs.' }));
  if (system && system.gpus && system.gpus.length) {
    page.append(h('section', { class: 'card' }, h('h2', { text: 'Graphics' }), ...system.gpus.map((g) => h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: g.name })), h('span', { class: 'muted', text: g.dedicated_video_memory_mb ? `${g.dedicated_video_memory_mb} MB video memory` : 'Integrated' })))));
  }
  page.append(h('section', { class: 'card' }, h('h2', { text: 'Video codecs' }), await codecTable(VIDEO, 'video', system)));
  page.append(h('section', { class: 'card' }, h('h2', { text: 'Audio codecs' }), await codecTable(AUDIO, 'audio', system)));
  const ct = h('table', { class: 'grid' }, h('tr', null, h('th', { text: 'Container' }), h('th', { text: 'Playable as file' })));
  for (const [name, type] of CONTAINERS) ct.append(h('tr', null, h('td', { text: name }), h('td', null, yes(canPlay(type) !== ''))));
  page.append(h('section', { class: 'card' }, h('h2', { text: 'Containers' }), ct));
  const uhd = await capabilities('video/webm; codecs="vp09.00.51.08"', 'video');
  const av1uhd = await capabilities('video/mp4; codecs="av01.0.12M.08"', 'video');
  page.append(h('section', { class: 'card' }, h('h2', { text: '4K playback' }),
    h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'VP9 2160p' })), yes(uhd && uhd.supported, uhd && uhd.powerEfficient ? 'Hardware' : 'Software')),
    h('div', { class: 'setting' }, h('div', { class: 'txt' }, h('div', { class: 't', text: 'AV1 2160p' })), yes(av1uhd && av1uhd.supported, av1uhd && av1uhd.powerEfficient ? 'Hardware' : 'Software'))));
  if (system && system.error) page.append(h('p', { class: 'muted', text: system.error }));
  main.replaceChildren(page);
}

bootPage(render);
