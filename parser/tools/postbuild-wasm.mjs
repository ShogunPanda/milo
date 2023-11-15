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

js = js.replace(
  'class Parser {',
  `
  const OFFSETS_SIZE = 2049 * 3 * 4;
    const OFFSETS_PADDING = 32768;
    const INITIAL_INPUT_SIZE = 1024 * 64;

    class Parser {
      static create(id = 0) {
        const sharedBuffer = module.exports.malloc(OFFSETS_PADDING + INITIAL_INPUT_SIZE, 1) >>> 0
        const parser = new Parser(id, sharedBuffer + OFFSETS_PADDING, sharedBuffer)

        parser.context = {}
        parser.offsets = new Uint32Array(wasm.memory.buffer, sharedBuffer, OFFSETS_SIZE)
        parser.input = new Uint8Array(wasm.memory.buffer, sharedBuffer + OFFSETS_PADDING, INITIAL_INPUT_SIZE)

        return parser
      }
  `
)

js = js.replace(
  'parse(limit) {',
  `
    parse(data, limit) {
      if (this.offsets.byteLength === 0 || limit > this.input.length) {
        const sharedBuffer = module.exports.malloc(OFFSETS_PADDING + limit, 1) >>> 0

        this.offsets = new Uint32Array(wasm.memory.buffer, sharedBuffer, OFFSETS_SIZE / 4)
        this.input = new Uint8Array(wasm.memory.buffer, sharedBuffer + OFFSETS_PADDING, limit)
        this.offsetsBuffer = sharedBuffer
        this.inputBuffer = sharedBuffer + OFFSETS_PADDING
      }

      this.input.set(data)
      this.context.input = data
  `
)

js = await prettier.format(js, { ...prettierConfig, parser: 'babel' })
// ts = await prettier.format(ts, { ...prettierConfig, parser: 'babel-ts' })

// TODO@PI: TS signature

await writeFile(javascriptPath, js, 'utf-8')
await writeFile(typescriptPath, ts, 'utf-8')
