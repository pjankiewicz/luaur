// Replay fuzz inputs through a wasm32-wasip1 standalone fuzz target under node's
// WASI, to find crashes that 64-bit native doesn't surface (32-bit pointers /
// different allocator). One WASI instance per input so a trap isolates the file.
//   node wasm_replay.mjs <module.wasm> <input-dir> [limit]
import { WASI } from 'node:wasi';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';

const [, , wasmPath, inputDir, limitArg] = process.argv;
const limit = parseInt(limitArg || '100000', 10);

const mod = await WebAssembly.compile(readFileSync(wasmPath));

let files = readdirSync(inputDir)
  .filter((f) => !f.startsWith('README'))
  .map((f) => path.join(inputDir, f))
  .filter((f) => statSync(f).isFile())
  .slice(0, limit);

let ok = 0, trapped = 0;
const traps = [];
for (const full of files) {
  const wasi = new WASI({
    version: 'preview1',
    args: ['run', '/in/' + path.basename(full)],
    env: {},
    preopens: { '/in': inputDir },
    returnOnExit: true, // exit(N) returns N instead of throwing
  });
  try {
    const instance = await WebAssembly.instantiate(mod, wasi.getImportObject());
    const code = wasi.start(instance);
    if (code && code !== 0) {
      trapped++;
      traps.push({ file: path.basename(full), kind: `exit ${code}` });
    } else ok++;
  } catch (e) {
    // wasm trap (unreachable/abort from a LUAU_ASSERT / UB) -> exception here
    trapped++;
    traps.push({ file: path.basename(full), kind: String(e.message || e).split('\n')[0] });
  }
}
console.log(`ran ${files.length}: ok=${ok} trapped/nonzero=${trapped}`);
for (const t of traps.slice(0, 30)) console.log('  >>', t.file, '::', t.kind);
