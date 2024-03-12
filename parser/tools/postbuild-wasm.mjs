import { readFile, writeFile } from 'node:fs/promises'
import prettier from 'prettier'
import prettierConfig from './prettier.config.cjs'

const buildInfoPath = new URL('../target/buildinfo.json', import.meta.url)
const profile = process.argv[2]
const javascriptPath = new URL(`../dist/wasm/${profile}/milo.js`, import.meta.url)
const typescriptPath = new URL(`../dist/wasm/${profile}/milo.d.ts`, import.meta.url)

const flags = ['DEBUG', 'ALL_CALLBACKS']
const configuration = Object.fromEntries(process.argv[3].split(',').map(p => p.split(':')))

const { constants } = JSON.parse(await readFile(buildInfoPath, 'utf-8'))

// Generate constants for JavaScript
let js = await readFile(javascriptPath, 'utf-8')
let ts = await readFile(typescriptPath, 'utf-8')

for (const [name, value] of Object.entries(constants)) {
  js += `module.exports.${name} = ${value}\n`
  ts += `export declare const ${name}: number = ${value}\n`
}

js += `module.exports.malloc = wasm.__wbindgen_malloc ?? wasm.__wbindgen_export_0\n`
ts += `export function malloc(a: number, b: number): number\n`

for (const flag of flags) {
  js += `module.exports.FLAGS_${flag} = ${configuration[flag]}\n`
  ts += `export declare const FLAGS_${flag}: boolean = ${configuration[flag]}\n`
}

/*
  TODO@PI: Use old undici buffer technique to grow both buffers for inputs and offsets.
  Pass them as arguments to internal parse
*/
js = js.replace(
  'class Parser {',
  `
    class Parser {
      static create(id = 0) {
        const parser = new Parser(id)

        parser.context = {
          inputBuffer: parser.inputBuffer,
          offsetsBuffer: parser.offsetsBuffer
        }

        return parser
      }
  `
)

js = js.replace(
  'parse(limit) {',
  `
    parse(data, limit) {
      this.context.inputBuffer.set(data)
      this.context.input = data
  `
)

js = await prettier.format(js, { ...prettierConfig, parser: 'babel' })
// ts = await prettier.format(ts, { ...prettierConfig, parser: 'babel-ts' })

// TODO@PI: TS signature

await writeFile(javascriptPath, js, 'utf-8')
await writeFile(typescriptPath, ts, 'utf-8')
