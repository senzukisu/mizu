// Checks the HTML/JS interface without a browser:
//   node scripts/check-ui.mjs
// 1. every script parses;
// 2. no translation key appears twice in the Russian dictionary (the later one silently wins);
// 3. no page script declares a top-level name that a shared script already declares — classic
//    scripts share one global scope, so a clash throws and the whole page stays blank.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { execFileSync } from 'node:child_process';

const root = new URL('..', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1');
const ui = join(root, 'assets', 'ui');
let failures = 0;
const fail = (msg) => {
  failures++;
  console.error(`✗ ${msg}`);
};

function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    return statSync(p).isDirectory() ? walk(p) : p.endsWith('.js') ? [p] : [];
  });
}
const files = walk(ui);

// 1. Syntax.
for (const f of files) {
  try {
    execFileSync(process.execPath, ['--check', f], { stdio: 'pipe' });
  } catch (e) {
    fail(`syntax: ${relative(root, f)}\n${String(e.stderr || e.message).split('\n').slice(0, 4).join('\n')}`);
  }
}

// 2. Duplicate dictionary keys.
const i18n = readFileSync(join(ui, 'shared', 'i18n.js'), 'utf8');
const keyRe = /^ {4}('(?:[^'\\]|\\.)*'|"(?:[^"\\]|\\.)*"):/gm;
const seen = new Set();
for (const m of i18n.matchAll(keyRe)) {
  if (seen.has(m[1])) fail(`i18n: duplicate key ${m[1]}`);
  seen.add(m[1]);
}

// 3. Top-level name clashes between shared scripts and each surface's own script.
const declRe = /^(?:const|let|var|function|class)\s+([A-Za-z_$][\w$]*)/gm;
const names = (src) => new Set([...src.matchAll(declRe)].map((m) => m[1]));
const shared = new Map();
for (const f of files.filter((f) => f.includes(join('ui', 'shared')))) {
  for (const n of names(readFileSync(f, 'utf8'))) {
    if (shared.has(n)) fail(`globals: ${n} declared in both ${shared.get(n)} and ${relative(ui, f)}`);
    shared.set(n, relative(ui, f));
  }
}
for (const f of files.filter((f) => !f.includes(join('ui', 'shared')))) {
  for (const n of names(readFileSync(f, 'utf8'))) {
    if (shared.has(n)) fail(`globals: ${relative(ui, f)} redeclares ${n} from ${shared.get(n)}`);
  }
}

if (failures) {
  console.error(`\n${failures} problem(s) in the interface scripts.`);
  process.exit(1);
}
console.log(`✓ ${files.length} interface scripts: syntax, dictionary and globals OK`);
