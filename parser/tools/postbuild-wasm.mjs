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

js = await prettier.format(js, { ...prettierConfig, parser: 'babel' })

// TODO@PI: TS signature
// ts = await prettier.format(ts, { ...prettierConfig, parser: 'babel-ts' })

await writeFile(javascriptPath, js, 'utf-8')
await writeFile(typescriptPath, ts, 'utf-8')
