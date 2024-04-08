/* eslint-disable no-unused-vars,camelcase,no-undef */

const textEncoder = new TextEncoder()
const textDecoder = new TextDecoder()

function loadWASM() {
  return require('node:fs').readFileSync(require('node:path').resolve(__dirname, 'milo.wasm'))
}

function logger(context, raw) {
  const len = Number(BigInt.asUintN(32, raw))
  const ptr = Number(raw >> 32n)

  console.error(textDecoder.decode(new Uint8Array(context.memory.buffer, ptr, len)))
}

function alloc(len) {
  return this.alloc(len) >>> 0
}

function dealloc(ptr) {
  return this.dealloc(ptr)
}

function create() {
  return this.create() >>> 0
}

function destroy(context, parser) {
  this.destroy(parser)
}

function parse(parser, data, limit) {
  return this.parse(parser, data, limit) >>> 0
}

function fail(parser, code, description) {
  const len = description.length
  const ptr = this.alloc(len)
  const buffer = new Uint8Array(this.memory.buffer, ptr, len)
  textEncoder.encodeInto(description, buffer)

  this.fail(parser, code, ptr, len)
  this.dealloc(ptr, len)
}

function getErrorDescription(parser) {
  const raw = this.get_error_description_raw(parser)
  const len = Number(BigInt.asUintN(32, raw))
  const ptr = Number(raw >> 32n)

  return textDecoder.decode(new Uint8Array(this.memory.buffer, ptr, len))
}

function getCallbackError(state, parser) {
  return state[parser][$milo_callback_error_index]
}

function $milo_getter_getMode(number) { }
function $milo_getter_isPaused(bool) { }
function $milo_getter_manageUnconsumed(bool) { }
function $milo_getter_continueWithoutData(bool) { }
function $milo_getter_isConnect(bool) { }
function $milo_getter_skipBody(bool) { }
function $milo_getter_getState(number) { }
function $milo_getter_getPosition(number) { }
function $milo_getter_getParsed(bigint) { }
function $milo_getter_getErrorCode(number) { }
function $milo_getter_getMessageType(number) { }
function $milo_getter_getMethod(number) { }
function $milo_getter_getStatus(number) { }
function $milo_getter_getVersionMajor(number) { }
function $milo_getter_getVersionMinor(number) { }
function $milo_getter_getConnection(number) { }
function $milo_getter_getContentLength(bigint) { }
function $milo_getter_getChunkSize(bigint) { }
function $milo_getter_getRemainingContentLength(bigint) { }
function $milo_getter_getRemainingChunkSize(bigint) { }
function $milo_getter_hasContentLength(bool) { }
function $milo_getter_hasChunkedTransferEncoding(bool) { }
function $milo_getter_hasUpgrade(bool) { }
function $milo_getter_hasTrailers(bool) { }

function $milo_setter_setContinueWithoutData() { }
function $milo_setter_setIsConnect() { }
function $milo_setter_setManageUnconsumed() { }
function $milo_setter_setMode() { }
function $milo_setter_setSkipBody() { }

function $milo_enum_MessageTypes(MESSAGE_TYPE_) { }
function $milo_enum_Errors(ERROR_) { }
function $milo_enum_Methods(METHOD_) { }
function $milo_enum_Connections(CONNECTION_) { }
function $milo_enum_Callbacks(CALLBACK_) { }
function $milo_enum_States(STATE_) { }

function noop() { }

const wasmModule = new WebAssembly.Module(loadWASM())

function setup(env = {}) {
  // Create the WASM instance
  const instance = new WebAssembly.Instance(wasmModule, {
    env: {
      logger: noop,
      $milo_callbacks,
      ...env
    }
  })
  const wasm = instance.exports

  const milo = {
    $milo_version,
    memory: wasm.memory,
    alloc: alloc.bind(wasm),
    dealloc: dealloc.bind(wasm),
    create: create.bind(wasm),
    destroy: destroy.bind(wasm),
    parse: parse.bind(wasm),
    fail: fail.bind(wasm),
    $milo_wasm: {
      dealloc,
      clear,
      finish,
      pause,
      reset,
      resume
    },
    $milo_getters,
    getErrorDescription: getErrorDescription.bind(wasm),
    $milo_setters,
    $milo_enums,
    $milo_constants,
    FLAG_DEBUG: $milo_flag_debug
  }

  $milo_start()
  return milo
}

function simpleCreate(spans, create) {
  const parser = create()
  spans[parser] = []
  return parser
}

function simpleDestroy(spans, destroy) {
  spans[parser] = undefined
  destroy(parser)
}

function simpleCallback(spans, type, parser, at, len) {
  spans[parser].push([type, at, len])
}

function simpleParser() {
  const spans = {}

  const milo = setup({
    $milo_simple_callbacks
  })

  milo.spans = spans
  milo.create = simpleCreate.bind(null, spans, milo.create)
  milo.destroy = simpleDestroy.bind(null, spans, milo.destroy)

  return milo
}

module.exports = { wasmModule, logger, setup, simple: simpleParser() }
