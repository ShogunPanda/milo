import { cp, mkdir, readFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { format } from 'prettier'
import prettierConfig from './prettier.config.js'

const enums = {
  ERROR: 'Errors',
  METHOD: 'Methods',
  CALLBACK: 'Callbacks',
  CALLBACK_ACTIVE: 'CallbackActives',
  STATE: 'States'
}

const getters = {
  isAutodetect: ['bool', 'is_autodetect'],
  isRequest: ['bool', 'is_request'],
  isPaused: ['bool', 'is_paused'],
  shouldManageUnconsumed: ['bool', 'should_manage_unconsumed'],
  getMaxStartLineLength: ['number', 'get_max_start_line_length'],
  getMaxHeaderLength: ['number', 'get_max_header_length'],
  shouldContinueWithoutData: ['bool', 'should_continue_without_data'],
  isConnect: ['bool', 'is_connect'],
  shouldSkipBody: ['bool', 'should_skip_body'],
  getState: ['number', 'get_state'],
  getPosition: ['number', 'get_position'],
  getParsed: ['bigint', 'get_parsed'],
  getErrorCode: ['number', 'get_error_code'],
  getMethod: ['number', 'get_method'],
  getStatus: ['number', 'get_status'],
  getVersionMajor: ['number', 'get_version_major'],
  getVersionMinor: ['number', 'get_version_minor'],
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
  setShouldManageUnconsumed: 'set_should_manage_unconsumed',
  setMaxStartLineLength: 'set_max_start_line_length',
  setMaxHeaderLength: 'set_max_header_length',
  setShouldSkipBody: 'set_should_skip_body',
  setActiveCallbacks: 'set_active_callbacks'
}

function getCallbacks(constants) {
  return Object.entries(constants).filter(c => c[0].startsWith('CALLBACK_') && !c[0].startsWith('CALLBACK_ACTIVE'))
}

function generateEnums(constants) {
  let replacement = ''
  for (let [selector, name] of Object.entries(enums)) {
    selector += '_'
    let matching = Object.keys(constants).filter(c => c.startsWith(selector))

    let suffix = ''
    if (selector === 'CALLBACK_') {
      matching = matching.filter(c => !c.startsWith('CALLBACK_ACTIVE'))
    } else if (selector === 'CALLBACK_ACTIVE_') {
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

function generateEnumsLists() {
  return Object.values(enums).join(',') + ','
}

function generateConstants(constants, flags) {
  return Object.entries(constants)
    .map(([k, v]) => {
      let value = v.toString()

      if (k === 'DEBUG') {
        value = flags.debug ?? value
      } else if (k.startsWith('CALLBACK_ACTIVE_')) {
        value += 'n'
      }

      return `${k}: ${value},`
    })
    .join('\n')
}

function generateGetters() {
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

function generateSetters() {
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

function generateGettersList() {
  return (
    Object.keys(getters)
      .map(g => `${g}: ${g}.bind(wasm)`)
      .join(',') + ','
  )
}

function generateSettersList() {
  return (
    Object.keys(setters)
      .map(g => `${g}: ${g}.bind(wasm)`)
      .join(',') + ','
  )
}

function generateNoopCallbacks(constants) {
  return getCallbacks(constants)
    .map(c => `${c[0].replace('CALLBACK_', '').toLowerCase()}: noop,`)
    .join('\n')
}

function generateSimpleCallbacks(constants) {
  return getCallbacks(constants)
    .map(
      c => `${c[0].replace('CALLBACK_', '').toLowerCase()}(parser, at, len) { spans[parser].push([${c[1]}, at, len]) },`
    )
    .join('\n')
}

async function generateModule(profile, version, flags, constants, loader) {
  const template = await readFile(new URL('../src/wasm/template.js', import.meta.url), 'utf-8')

  const replaced = template.replaceAll(/\/\* REPLACE: (\S+) \*\//g, (marker, id) => {
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
        return generateConstants(constants, flags)
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

  return format(replaced, { ...prettierConfig, parser: 'babel' })
}

// TODO@PI: TypeScript
async function main() {
  const { version, constants } = JSON.parse(
    await readFile(new URL('../target/buildinfo.json', import.meta.url), 'utf-8')
  )

  const profile = process.argv[2]
  const flags = Object.fromEntries(process.argv[3].split(',').map(p => p.split(':').map(s => s.toLowerCase())))

  // Generate the required files
  const wasm = await readFile(new URL(`../dist/wasm/${profile}/milo.wasm`, import.meta.url), 'base64')
  const unbundled = await generateModule(
    profile,
    version,
    flags,
    constants,
    `import { readFileSync } from 'node:fs'\n\nexport const wasmModule = new WebAssembly.Module(readFileSync(new URL('./milo.wasm', import.meta.url)))
    `
  )
  const bundled = await generateModule(
    profile,
    version,
    flags,
    constants,
    `export const wasmModule = new WebAssembly.Module(Uint8Array.from(globalThis.atob('${wasm}'), c => c.codePointAt(0)))
    `
  )

  // Open the package.json and update the version
  const packageJson = JSON.parse(await readFile(new URL('../src/wasm/package.json', import.meta.url), 'utf-8'))
  const rootFolder = fileURLToPath(new URL(`../dist/wasm/${profile}/${packageJson.name}`, import.meta.url))
  packageJson.version = Object.values(version).join('.')

  // Write files
  await mkdir(rootFolder, { recursive: true })
  await writeFile(resolve(rootFolder, 'package.json'), JSON.stringify(packageJson, null, 2), 'utf-8')
  await writeFile(resolve(rootFolder, 'unbundled.js'), unbundled, 'utf-8')
  await writeFile(resolve(rootFolder, 'index.js'), bundled, 'utf-8')
  await cp(new URL(`../dist/wasm/${profile}/milo.wasm`, import.meta.url), resolve(rootFolder, 'milo.wasm'))
  await cp(new URL(`../dist/wasm/${profile}/milo.wasm`, import.meta.url), resolve(rootFolder, 'milo.wasm'))

  // Copy other Markdown files from root
  for (const file of ['CODE_OF_CONDUCT', 'LICENSE', 'README']) {
    await cp(new URL(`../../${file}.md`, import.meta.url), resolve(rootFolder, `${file}.md`))
  }
}

await main()
