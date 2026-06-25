import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { format } from 'prettier'
import prettierConfig from '../prettier.config.js'
import { getBuildInfo } from './buildinfo.js'

const enums = {
  ERROR: 'Errors',
  METHOD: 'Methods',
  CALLBACK: 'Callbacks',
  CALLBACK_ACTIVE: 'CallbackActives',
  EVENT: 'Events',
  EVENT_ACTIVE: 'EventActives',
  STATE: 'States',
  PARSER_FIELD: 'ParserFields'
}

const getters = {
  isAutodetect: ['bool', 'is_autodetect'],
  isRequest: ['bool', 'is_request'],
  isPaused: ['bool', 'is_paused'],
  shouldManageUnconsumed: ['bool', 'should_manage_unconsumed'],
  shouldSuspendAfterHeaders: ['bool', 'should_suspend_after_headers'],
  getMaxStartLineLength: ['number', 'get_max_start_line_length'],
  getMaxHeaderLength: ['number', 'get_max_header_length'],
  getMaxBodyPayload: ['bigint', 'get_max_body_payload'],
  shouldContinueWithoutData: ['bool', 'should_continue_without_data'],
  isConnect: ['bool', 'is_connect'],
  isDebug: ['bool', 'is_debug'],
  shouldSkipBody: ['bool', 'should_skip_body'],
  getState: ['number', 'get_state'],
  getPosition: ['number', 'get_position'],
  getParsed: ['bigint', 'get_parsed'],
  getErrorCode: ['number', 'get_error_code'],
  getMethod: ['number', 'get_method'],
  getStatus: ['number', 'get_status'],
  hasConnectionClose: ['bool', 'has_connection_close'],
  hasConnectionUpgrade: ['bool', 'has_connection_upgrade'],
  getContentLength: ['bigint', 'get_content_length'],
  getChunkSize: ['bigint', 'get_chunk_size'],
  getRemainingContentLength: ['bigint', 'get_remaining_content_length'],
  getRemainingChunkSize: ['bigint', 'get_remaining_chunk_size'],
  hasContentLength: ['bool', 'has_content_length'],
  hasTransferEncoding: ['bool', 'has_transfer_encoding'],
  hasChunkedTransferEncoding: ['bool', 'has_chunked_transfer_encoding'],
  hasUpgrade: ['bool', 'has_upgrade'],
  hasTrailers: ['bool', 'has_trailers'],
  getErrorDescription: ['string', 'get_error_description_raw']
}

const setters = {
  setShouldAutodetect: 'set_should_autodetect',
  setShouldContinueWithoutData: 'set_should_continue_without_data',
  setIsRequest: 'set_is_request',
  setIsConnect: 'set_is_connect',
  setDebug: 'set_debug',
  setShouldManageUnconsumed: 'set_should_manage_unconsumed',
  setShouldSuspendAfterHeaders: 'set_should_suspend_after_headers',
  setMaxStartLineLength: 'set_max_start_line_length',
  setMaxHeaderLength: 'set_max_header_length',
  setMaxBodyPayload: 'set_max_body_payload',
  setShouldSkipBody: 'set_should_skip_body',
  setActiveCallbacks: 'set_active_callbacks',
  setActiveEvents: 'set_active_events'
}

function getCallbacks (constants) {
  return Object.entries(constants).filter(c => c[0].startsWith('CALLBACK_') && !c[0].startsWith('CALLBACK_ACTIVE'))
}

function generateEnums (constants) {
  let replacement = ''
  for (let [selector, name] of Object.entries(enums)) {
    selector += '_'
    let matching = Object.keys(constants).filter(c => c.startsWith(selector))

    let suffix = ''
    if (selector === 'CALLBACK_') {
      matching = matching.filter(c => !c.startsWith('CALLBACK_ACTIVE'))
    } else if (selector === 'EVENT_') {
      matching = matching.filter(c => !c.startsWith('EVENT_ACTIVE_'))
    } else if (selector === 'CALLBACK_ACTIVE_' || selector === 'EVENT_ACTIVE_') {
      suffix = 'n'
    }

    replacement += `
const ${name} = Object.freeze({
${matching.map(k => `${k.replace(selector, '')}: ${constants[k]}${suffix}`).join(',\n')},
${matching.map(k => `${constants[k]}${suffix}: '${k.replace(selector, '')}'`).join(',\n')}
})
    `
  }

  return replacement
}

function generateEnumsLists () {
  return Object.values(enums).join(',') + ','
}

function generateConstants (constants) {
  return Object.entries(constants)
    .map(([k, v]) => {
      let value = v.toString()

      if (k.startsWith('CALLBACK_ACTIVE_') || k.startsWith('EVENT_ACTIVE_')) {
        value += 'n'
      }

      return `${k}: ${value},`
    })
    .join('\n')
}

function generateGetters () {
  let replacement = ''

  for (const [getter, [type, rawGetter]] of Object.entries(getters)) {
    let body = ''
    switch (type) {
      case 'number':
        body = 'return $ >>> 0'
        break
      case 'bigint':
        body = 'return BigInt.asUintN(64, $)'
        break
      case 'bool':
        body = 'return $ !== 0'
        break
      case 'string':
        body = `
        const raw = $
        const len = Number(BigInt.asUintN(32, raw))
        const ptr = Number(raw >> 32n)
        return textDecoder.decode(new Uint8Array(this.memory.buffer, ptr, len))
        `
        break
    }

    replacement += `
function ${getter}(parser) {
  ${body.replace('$', `this.${rawGetter}(parser)`)}
}
    `
  }

  return replacement
}

function generateSetters () {
  let replacement = ''

  for (const [setter, rawSetter] of Object.entries(setters)) {
    replacement += `
function ${setter}(parser, value) {
  this.${rawSetter}(parser, value)
}
    `
  }

  return replacement
}

function generateGettersList () {
  return (
    Object.keys(getters)
      .map(g => `${g}: ${g}.bind(wasm)`)
      .join(',') + ','
  )
}

function generateSettersList () {
  return (
    Object.keys(setters)
      .map(g => `${g}: ${g}.bind(wasm)`)
      .join(',') + ','
  )
}

function generateNoopCallbacks (constants) {
  return getCallbacks(constants)
    .map(c => `${c[0].replace('CALLBACK_', '').toLowerCase()}: noop,`)
    .join('\n')
}

function generateSimpleCallbacks (constants) {
  return getCallbacks(constants)
    .map(
      c => `${c[0].replace('CALLBACK_', '').toLowerCase()}(parser, at, len) { spans[parser].push([${c[1]}, at, len]) },`
    )
    .join('\n')
}

async function generateModule (profile, version, constants, loader, moduleFormat) {
  const template = await readFile(new URL('../parser/src/wasm/template.js', import.meta.url), 'utf-8')

  // The template is shared by bundled/unbundled and ESM/CJS outputs; placeholders keep the runtime code identical.
  let replaced = template.replaceAll(/\/\* REPLACE: (\S+) \*\//g, (marker, id) => {
    switch (id) {
      case 'module':
        return loader
      case 'version':
        return `version: ${JSON.stringify(version, null, 2)},`
      case 'enums':
        return generateEnums(constants)
      case 'enums:list':
        return generateEnumsLists()
      case 'constants':
        return generateConstants(constants)
      case 'getters':
        return generateGetters()
      case 'setters':
        return generateSetters()
      case 'getters:list':
        return generateGettersList()
      case 'setters:list':
        return generateSettersList()
      case 'callbacks:noop':
        return generateNoopCallbacks(constants)
      case 'callbacks:simple':
        return generateSimpleCallbacks(constants)
      case 'start':
        return profile === 'debug' ? 'wasm.__start()' : ''
      default:
        console.warn(`Unsupported placeholder type ${id}`)
        return marker
    }
  })

  if (moduleFormat === 'commonjs') {
    replaced = replaced.replaceAll(/^export function /gm, 'function ')
    replaced += '\nmodule.exports = { wasmModule, noop, setup, simple }\n'
  }

  return format(replaced, { ...prettierConfig, parser: 'babel' })
}

function generateCommonjsPackageJson (packageJson) {
  const cjsPackageJson = JSON.parse(JSON.stringify(packageJson))

  cjsPackageJson.name = '@perseveranza-pets/milo-cjs'
  cjsPackageJson.type = 'commonjs'

  return cjsPackageJson
}

async function generateVariant (profile, version, constants, rootFolder, variant, moduleFormat) {
  const wasmFile = `${variant}.wasm`
  const sourceFolder = resolve(rootFolder, 'src', variant)
  const wasm = await readFile(new URL(`../dist/wasm/${profile}/binary/${wasmFile}`, import.meta.url), 'base64')
  const isCommonjs = moduleFormat === 'commonjs'
  const unbundledLoader = isCommonjs
    ? `const { readFileSync } = require('node:fs')
const { join } = require('node:path')

const wasmModule = new WebAssembly.Module(readFileSync(join(__dirname, '../../binary/${wasmFile}')))
    `
    : `import { readFileSync } from 'node:fs'\n\nexport const wasmModule = new WebAssembly.Module(readFileSync(new URL('../../binary/${wasmFile}', import.meta.url)))
    `
  const bundledLoader = isCommonjs
    ? `const wasmModule = new WebAssembly.Module(Buffer.from('${wasm}', 'base64'))
    `
    : `export const wasmModule = new WebAssembly.Module(Uint8Array.from(globalThis.atob('${wasm}'), c => c.codePointAt(0)))
    `
  const unbundled = await generateModule(profile, version, constants, unbundledLoader, moduleFormat)
  const bundled = await generateModule(profile, version, constants, bundledLoader, moduleFormat)

  await mkdir(sourceFolder, { recursive: true })
  await writeFile(resolve(sourceFolder, 'unbundled.js'), unbundled, 'utf-8')
  await writeFile(resolve(sourceFolder, 'index.js'), bundled, 'utf-8')
}

// TODO@PI: TypeScript
async function main () {
  const { version, constants } = await getBuildInfo()

  const profile = process.argv[2]

  // Open the package.json and update the version
  const packageJson = JSON.parse(await readFile(new URL('../parser/src/wasm/package.json', import.meta.url), 'utf-8'))
  const rootFolder = fileURLToPath(new URL(`../dist/wasm/${profile}/package`, import.meta.url))
  const cjsRootFolder = fileURLToPath(new URL(`../dist/wasm/${profile}/package-cjs`, import.meta.url))
  packageJson.version = version.raw
  const cjsPackageJson = generateCommonjsPackageJson(packageJson)

  // Write files
  await rm(rootFolder, { recursive: true, force: true })
  await rm(cjsRootFolder, { recursive: true, force: true })
  await mkdir(rootFolder, { recursive: true })
  await mkdir(cjsRootFolder, { recursive: true })
  await cp(new URL(`../dist/wasm/${profile}/binary`, import.meta.url), resolve(rootFolder, 'binary'), {
    recursive: true
  })
  await cp(new URL(`../dist/wasm/${profile}/binary`, import.meta.url), resolve(cjsRootFolder, 'binary'), {
    recursive: true
  })
  await generateVariant(profile, version, constants, rootFolder, 'simd', 'module')
  await generateVariant(profile, version, constants, rootFolder, 'no-simd', 'module')
  await generateVariant(profile, version, constants, cjsRootFolder, 'simd', 'commonjs')
  await generateVariant(profile, version, constants, cjsRootFolder, 'no-simd', 'commonjs')
  await writeFile(resolve(rootFolder, 'package.json'), JSON.stringify(packageJson, null, 2), 'utf-8')
  await writeFile(resolve(cjsRootFolder, 'package.json'), JSON.stringify(cjsPackageJson, null, 2), 'utf-8')

  // Copy other Markdown files from root
  for (const file of ['CODE_OF_CONDUCT', 'LICENSE', 'README']) {
    await cp(new URL(`../${file}.md`, import.meta.url), resolve(rootFolder, `${file}.md`))
    await cp(new URL(`../${file}.md`, import.meta.url), resolve(cjsRootFolder, `${file}.md`))
  }
}

await main()
