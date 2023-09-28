import { readFile, writeFile } from 'node:fs/promises'

const buildInfoPath = new URL('../target/buildinfo.json', import.meta.url)
const profile = process.argv[2]
const javascriptPath = new URL(`../dist/wasm/${profile}/milo.js`, import.meta.url)
const typescriptPath = new URL(`../dist/wasm/${profile}/milo.d.ts`, import.meta.url)

const { constants, flags } = JSON.parse(await readFile(buildInfoPath, 'utf-8'))

// Generate constants for JavaScript
let js = await readFile(javascriptPath, 'utf-8')
let ts = await readFile(typescriptPath, 'utf-8')

for (const [name, value] of Object.entries(constants)) {
  js += `module.exports.${name} = ${value};\n`
  ts += `export declare const ${name}: number = ${value};\n`
}

js += `module.exports.malloc = wasm.__wbindgen_malloc ?? wasm.__wbindgen_export_0;`
ts += `export function malloc(a: number, b: number): number;`

await writeFile(javascriptPath, js, 'utf-8')
await writeFile(typescriptPath, ts, 'utf-8')
