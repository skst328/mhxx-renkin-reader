// wasm32-wasip1 向けに作ったテストを Node の WASI で動かす (.cargo/config.toml の runner)。
// 使い方: node tests/wasi-run.mjs <テストの .wasm> [テストの引数...]
// テストが読むのは、このクレート (見本・比べるデータ) と charm-pics (CHARM_PICS、無ければリポジトリの隣) なので、
// その 2 つの場所だけを wasm から見えるようにする (同じ絶対パスで見せるので、テストの中のパスはそのまま使える)
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { WASI } from 'node:wasi';

const [wasmPath, ...args] = process.argv.slice(2);
const crate = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const charmPics = resolve(process.env.CHARM_PICS ?? resolve(crate, '../../charm-pics'));
const preopens = Object.fromEntries([crate, charmPics].map((d) => [d, d]));
// テストの既定のパス (クレートから ../../charm-pics) は見せた場所の外を通るので、解決したパスを渡す
const env = { ...process.env, CHARM_PICS: charmPics };
const wasi = new WASI({ version: 'preview1', args: [wasmPath, ...args], env, preopens, returnOnExit: true });
const module = await WebAssembly.compile(readFileSync(wasmPath));
const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
process.exit(wasi.start(instance));
