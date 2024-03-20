import { readFile, writeFile } from 'node:fs/promises'
import prettier from 'prettier'
import prettierConfig from './prettier.config.cjs'

const buildInfoPath = new URL('../target/buildinfo.json', import.meta.url)
const profile = process.argv[2]
const javascriptPath = new URL(`../dist/wasm/${profile}/milo.js`, import.meta.url)
const typescriptPath = new URL(`../dist/wasm/${profile}/milo.d.ts`, import.meta.url)

const flags = ['DEBUG']
const configuration = Object.fromEntries(process.argv[3].split(',').map(p => p.split(':')))

const { constants } = JSON.parse(await readFile(buildInfoPath, 'utf-8'))

// Generate constants for JavaScript
let js = await readFile(javascriptPath, 'utf-8')
let ts = await readFile(typescriptPath, 'utf-8')

// Prepend the wasm user code
// TODO@PI: Typescript Counterparts
let glue = await readFile(new URL('../src/wasm.js', import.meta.url), 'utf-8')

js += `\n\n/*--- MILO MODIFICATIONS ---*/\n\n${glue}\n`
ts += `\n\n/*--- MILO MODIFICATIONS ---*/\n\n`

// Add the callbacks setter
const callbacksTotal = Object.keys(constants)
  .filter(c => c.startsWith('CALLBACKS_'))
  .map(c => 'undefined')

js += `const emptyCallbacks = [${callbacksTotal.join(', ')}];`

for (const [name, value] of Object.entries(constants)) {
  if (!name.startsWith('CALLBACKS_')) {
    continue
  }

  let setter = `set_${name.slice(10)}`.toLowerCase().replaceAll(/_([a-z])/g, (_, l) => l.toUpperCase())

  js += `
\n
module.exports.${setter} = function ${setter}(parser, cb) {
  callbacksRegistry[parser] ??= structuredClone(emptyCallbacks);
  callbacksRegistry[parser][module.exports.${name}] = cb;
}
\n`
}

// Add all the exports
for (const [name, value] of Object.entries(constants)) {
  js += `module.exports.${name} = ${value}\n`
  ts += `export declare const ${name}: number = ${value}\n`
}

for (const flag of flags) {
  js += `module.exports.FLAGS_${flag} = ${configuration[flag]}\n`
  ts += `export declare const FLAGS_${flag}: boolean = ${configuration[flag]}\n`
}

js += `module.exports.memory = wasm.memory`
ts += `export declare const memory: WebAssembly.Memory`

js = js.replaceAll('wasm.destroy(raw)', 'callbacksRegistry[raw] = undefined; wasm.destroy(raw)')

js = await prettier.format(js, { ...prettierConfig, parser: 'babel' })

// TODO@PI: TS signature
// ts = await prettier.format(ts, { ...prettierConfig, parser: 'babel-ts' })

await writeFile(javascriptPath, js, 'utf-8')
await writeFile(typescriptPath, ts, 'utf-8')
