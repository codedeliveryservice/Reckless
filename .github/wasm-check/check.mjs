// Compares the strict wasm build's NNUE evals against the native binary.
//
// Usage: node check.mjs PKG_DIR NATIVE_BIN POSITIONS_TXT [TOLERANCE_CP]
//
// The strict wasm and native builds sit in different f32 rounding families
// (non-fused vs fused madd), at most 1cp apart in practice, hence the
// default tolerance. Real breakage shows up as hundreds of cp.
//
// Exit codes: 0 ok, 1 eval mismatch beyond tolerance, 2 usage/load failure.

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';

const [pkgDir, nativeBin, positionsPath, tolArg] = process.argv.slice(2);
if (!pkgDir || !nativeBin || !positionsPath) {
  console.error('usage: node check.mjs PKG_DIR NATIVE_BIN POSITIONS_TXT [TOLERANCE_CP]');
  process.exit(2);
}
const tolerance = tolArg === undefined ? 1 : Number(tolArg);

const fens = readFileSync(positionsPath, 'utf8')
  .split('\n').map(s => s.trim()).filter(s => s && !s.startsWith('#'));

// Native evals: one process, "position fen X" + "eval" per position. The
// final line of each eval block is "NNUE evaluation  +0.80 (White side)".
const input = fens.map(f => `position fen ${f}\neval\n`).join('') + 'quit\n';
const native = spawnSync(nativeBin, [], { input, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
if (native.status !== 0) {
  console.error(`native binary failed: status ${native.status}\n${native.stderr}`);
  process.exit(2);
}
const nativeEvals = [...native.stdout.matchAll(/NNUE evaluation\s+([+-]?\d+\.\d+)/g)]
  .map(m => Math.round(parseFloat(m[1]) * 100));
if (nativeEvals.length !== fens.length) {
  console.error(`expected ${fens.length} native evals, parsed ${nativeEvals.length}`);
  process.exit(2);
}

let glue, engine;
try {
  glue = await import(pathToFileURL(resolve(pkgDir, 'reckless.js')).href);
  glue.initSync({ module: readFileSync(resolve(pkgDir, 'reckless_bg.wasm')) });
  engine = new glue.Engine();
  engine.set_threads(1);
} catch (e) {
  console.error(`failed to load wasm package from ${pkgDir}: ${e}`);
  process.exit(2);
}

let failures = 0;
fens.forEach((fen, i) => {
  engine.set_position(fen);
  const stm = fen.split(/\s+/)[1];
  const wasmWhite = stm === 'b' ? -engine.evaluate() : engine.evaluate();
  const delta = Math.abs(wasmWhite - nativeEvals[i]);
  if (delta > tolerance) {
    failures++;
    console.error(`MISMATCH  native ${nativeEvals[i]}  wasm ${wasmWhite}  (delta ${delta})  ${fen}`);
  }
});

if (failures > 0) {
  console.error(`${failures}/${fens.length} positions beyond ${tolerance}cp of native`);
  process.exit(1);
}
console.log(`${fens.length}/${fens.length} positions within ${tolerance}cp of native`);
