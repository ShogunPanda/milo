import { readFile } from 'node:fs/promises'
import { wasmModule } from '../../../dist/wasm/release/package/src/simd/unbundled.js'

const TARGET_BYTES = 8n << 30n
const ERROR_NONE = 0
const HTTP_REQUEST = 1
const HTTP_RESPONSE = 2
const MILO_ACTIVE_CALLBACKS =
  4n | // ON_MESSAGE_START
  8n | // ON_MESSAGE_COMPLETE
  256n | // ON_URL
  2048n | // ON_STATUS
  8192n | // ON_HEADER_NAME
  16384n | // ON_HEADER_VALUE
  32768n | // ON_HEADERS
  8388608n // ON_DATA

function noop () {}

async function loadFixture (name, isRequest) {
  const raw = await readFile(new URL(`../../fixtures/${name}.txt`, import.meta.url))

  return {
    name,
    isRequest,
    payload: decodeFixture(raw)
  }
}

function decodeFixture (raw) {
  const decoded = []

  // Fixtures encode CRLF as text and separate rows with LF; rebuild the payload bytes used by parsers.
  let i = 0
  while (i < raw.length) {
    if (raw[i] === 0x0a) {
      i++
    } else if (
      i + 3 < raw.length &&
      raw[i] === 0x5c &&
      raw[i + 1] === 0x72 &&
      raw[i + 2] === 0x5c &&
      raw[i + 3] === 0x6e
    ) {
      decoded.push(0x0d, 0x0a)
      i += 4
    } else {
      decoded.push(raw[i])
      i++
    }
  }

  return Uint8Array.from(decoded)
}

function formatNumber (value, precision) {
  return value
    .toLocaleString('en-US', {
      minimumFractionDigits: precision,
      maximumFractionDigits: precision,
      useGrouping: true
    })
    .replaceAll(',', '_')
}

function padLeft (value, width) {
  return value.padStart(width, ' ')
}

function padRight (value, width) {
  return value.padEnd(width, ' ')
}

function printSeparator (parserWidth, iterationsWidth, mbWidth, opsWidth) {
  console.log(
    `| ${'-'.repeat(parserWidth)} | ${'-'.repeat(iterationsWidth)} | ${'-'.repeat(mbWidth)} | ${'-'.repeat(opsWidth)} |`
  )
}

function printResults (fixture, results) {
  results.sort((a, b) => a.bytesPerSecond - b.bytesPerSecond)

  const parserWidth = Math.max('Parser'.length, ...results.map(result => result.parser.length))
  const iterationsWidth = Math.max('Iterations'.length, ...results.map(result => result.iterations.length))
  const mbWidth = Math.max('MB/s'.length, ...results.map(result => result.mbPerSecond.length))
  const opsWidth = Math.max('Ops/s'.length, ...results.map(result => result.opsPerSecond.length))

  console.log(`### ${fixture.name}\n`)
  console.log(
    `| ${padRight('Parser', parserWidth)} | ${padLeft('Iterations', iterationsWidth)} | ${padLeft('MB/s', mbWidth)} | ${padLeft('Ops/s', opsWidth)} |`
  )
  printSeparator(parserWidth, iterationsWidth, mbWidth, opsWidth)

  for (const result of results) {
    console.log(
      `| ${padRight(result.parser, parserWidth)} | ${padLeft(result.iterations, iterationsWidth)} | ${padLeft(result.mbPerSecond, mbWidth)} | ${padLeft(result.opsPerSecond, opsWidth)} |`
    )
  }

  console.log('')
}

function createMilo () {
  return new WebAssembly.Instance(wasmModule, {
    env: {
      on_reset: noop,
      on_error: noop,
      on_message_complete: noop,
      on_response: noop,
      on_message_start: noop,
      on_request: noop,
      on_headers: noop,
      on_upgrade: noop,
      on_finish: noop,
      on_connect: noop,
      on_data: noop,
      on_body: noop,
      on_chunk: noop,
      on_trailers: noop,
      on_trailer_name: noop,
      on_trailer_value: noop,
      on_chunk_extension_name: noop,
      on_chunk_extension_value: noop,
      on_chunk_length: noop,
      on_header_name: noop,
      on_header_value: noop,
      on_protocol: noop,
      on_version: noop,
      on_status: noop,
      on_reason: noop,
      on_method: noop,
      on_url: noop
    }
  }).exports
}

function createLlhttp (wasmBytes) {
  const imports = {
    env: {
      wasm_on_headers_complete: noop,
      wasm_on_message_begin: noop,
      wasm_on_url: noop,
      wasm_on_status: noop,
      wasm_on_header_field: noop,
      wasm_on_header_value: noop,
      wasm_on_body: noop,
      wasm_on_message_complete: noop
    }
  }
  const module = new WebAssembly.Module(wasmBytes)
  const instance = new WebAssembly.Instance(module, imports)
  const llhttp = instance.exports

  llhttp._initialize()
  return llhttp
}

function copyToMemory (memory, ptr, payload) {
  new Uint8Array(memory.buffer, ptr, payload.length).set(payload)
}

function validateMilo (milo, fixture) {
  const parser = milo.create()
  const ptr = milo.alloc(fixture.payload.length)
  copyToMemory(milo.memory, ptr, fixture.payload)

  milo.set_should_autodetect(parser, false)
  milo.set_is_request(parser, fixture.isRequest)
  milo.set_active_callbacks(parser, MILO_ACTIVE_CALLBACKS)

  const consumed = milo.parse(parser, ptr, fixture.payload.length)
  const error = milo.get_error_code(parser)

  milo.destroy(parser)
  milo.dealloc(ptr, fixture.payload.length)

  if (consumed !== fixture.payload.length || error !== ERROR_NONE) {
    throw new Error(
      `Milo failed to parse fixture ${fixture.name}: consumed ${consumed} of ${fixture.payload.length} bytes, error ${error}`
    )
  }
}

function validateLlhttp (llhttp, fixture) {
  const parser = llhttp.llhttp_alloc(fixture.isRequest ? HTTP_REQUEST : HTTP_RESPONSE)
  const ptr = llhttp.malloc(fixture.payload.length)
  copyToMemory(llhttp.memory, ptr, fixture.payload)

  const error = llhttp.llhttp_execute(parser, ptr, fixture.payload.length)

  llhttp.llhttp_free(parser)
  llhttp.free(ptr)

  if (error !== 0) {
    throw new Error(`llhttp failed to parse fixture ${fixture.name}: error ${error}`)
  }
}

function benchmarkMilo (milo, fixture) {
  const iterations = Number(TARGET_BYTES / BigInt(fixture.payload.length))
  const total = iterations * fixture.payload.length
  const parser = milo.create()
  const ptr = milo.alloc(fixture.payload.length)
  copyToMemory(milo.memory, ptr, fixture.payload)

  milo.set_should_autodetect(parser, false)
  milo.set_is_request(parser, fixture.isRequest)
  milo.set_active_callbacks(parser, MILO_ACTIVE_CALLBACKS)

  const start = process.hrtime.bigint()
  let consumed = 0
  for (let i = 0; i < iterations; i++) {
    consumed += milo.parse(parser, ptr, fixture.payload.length)
  }
  const seconds = Number(process.hrtime.bigint() - start) / 1e9

  const error = milo.get_error_code(parser)

  milo.destroy(parser)
  milo.dealloc(ptr, fixture.payload.length)

  if (consumed !== total || error !== ERROR_NONE) {
    throw new Error(
      `Milo failed while benchmarking fixture ${fixture.name}: consumed ${consumed} of ${total} bytes, error ${error}`
    )
  }

  const bytesPerSecond = total / seconds
  return {
    parser: 'milo-wasm',
    iterations: formatNumber(iterations, 0),
    mbPerSecond: formatNumber(bytesPerSecond / (1024 * 1024), 2),
    opsPerSecond: formatNumber(iterations / seconds, 2),
    bytesPerSecond
  }
}

function benchmarkLlhttp (llhttp, fixture) {
  const iterations = Number(TARGET_BYTES / BigInt(fixture.payload.length))
  const total = iterations * fixture.payload.length
  const parser = llhttp.llhttp_alloc(fixture.isRequest ? HTTP_REQUEST : HTTP_RESPONSE)
  const ptr = llhttp.malloc(fixture.payload.length)
  copyToMemory(llhttp.memory, ptr, fixture.payload)

  const start = process.hrtime.bigint()
  let errors = 0
  for (let i = 0; i < iterations; i++) {
    errors += llhttp.llhttp_execute(parser, ptr, fixture.payload.length)
  }
  const seconds = Number(process.hrtime.bigint() - start) / 1e9

  llhttp.llhttp_free(parser)
  llhttp.free(ptr)

  if (errors !== 0) {
    throw new Error(`llhttp failed while benchmarking fixture ${fixture.name}: error ${errors}`)
  }

  const bytesPerSecond = total / seconds
  return {
    parser: 'llhttp-wasm',
    iterations: formatNumber(iterations, 0),
    mbPerSecond: formatNumber(bytesPerSecond / (1024 * 1024), 2),
    opsPerSecond: formatNumber(iterations / seconds, 2),
    bytesPerSecond
  }
}

const fixtures = [
  await loadFixture('seanmonstar_httparse', true),
  await loadFixture('nodejs_http_parser', true),
  await loadFixture('undici', false)
]
const milo = createMilo()
const llhttp = createLlhttp(await readFile(new URL('../../../tmp/external/llhttp_simd.wasm', import.meta.url)))

for (const fixture of fixtures) {
  validateMilo(milo, fixture)
  validateLlhttp(llhttp, fixture)

  printResults(fixture, [benchmarkMilo(milo, fixture), benchmarkLlhttp(llhttp, fixture)])
}
