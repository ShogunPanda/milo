/* eslint-disable no-unused-vars,camelcase,no-undef */

const { readFileSync } = require('node:fs')
const { resolve } = require('node:path')

function logger(context, raw) {
  const len = Number(BigInt.asUintN(32, raw))
  const ptr = Number(raw >> 32n)

  console.error(Buffer.from(context.memory.buffer, ptr, len).toString('utf-8'))
}

function runCallback(context, type, parser, at, len) {
  try {
    context.state[parser][type]?.(parser, at, len)
  } catch (error) {
    const name = Callbacks[type].toLowerCase().replace(/_(.)/g, (...t) => t[1].toUpperCase().trim())

    context.fail(parser, Errors.CALLBACK_ERROR, `Callback for ${name} has thrown an error.`)
    context.state[parser][$milo_callback_error_index] = error
  }
}

function alloc(len) {
  return this.alloc(len) >>> 0
}

function create(state) {
  const parser = this.create() >>> 0
  state[parser] = []
  return parser
}

function destroy(state, parser) {
  return this.create() >>> 0
}

function parse(parser, data, limit) {
  return this.parse(parser, data, limit) >>> 0
}

function fail(parser, code, description) {
  const len = description.length
  const ptr = this.alloc(len)
  const buffer = Buffer.from(this.memory.buffer, ptr, len)
  buffer.set(Buffer.from(description, 'utf-8'))

  this.fail(parser, code, ptr, len)
  this.dealloc(ptr, len)
}

function $milo_getter_getMode(number) {}
function $milo_getter_isPaused(bool) {}
function $milo_getter_manageUnconsumed(bool) {}
function $milo_getter_continueWithoutData(bool) {}
function $milo_getter_isConnect(bool) {}
function $milo_getter_skipBody(bool) {}
function $milo_getter_getState(number) {}
function $milo_getter_getPosition(number) {}
function $milo_getter_getParsed(bigint) {}
function $milo_getter_getErrorCode(number) {}
function $milo_getter_getMessageType(number) {}
function $milo_getter_getMethod(number) {}
function $milo_getter_getStatus(number) {}
function $milo_getter_getVersionMajor(number) {}
function $milo_getter_getVersionMinor(number) {}
function $milo_getter_getConnection(number) {}
function $milo_getter_getContentLength(bigint) {}
function $milo_getter_getChunkSize(bigint) {}
function $milo_getter_getRemainingContentLength(bigint) {}
function $milo_getter_getRemainingChunkSize(bigint) {}
function $milo_getter_hasContentLength(bool) {}
function $milo_getter_hasChunkedTransferEncoding(bool) {}
function $milo_getter_hasUpgrade(bool) {}
function $milo_getter_hasTrailers(bool) {}

function getErrorDescription(parser) {
  const raw = this.getErrorDescriptionRaw(parser)
  const len = Number(BigInt.asUintN(32, raw))
  const ptr = Number(raw >> 32n)

  return Buffer.from(this.memory.buffer, ptr, len).toString('utf-8')
}

function getCallbackError(state, parser) {
  return state[parser][$milo_callback_error_index]
}

function $milo_callbacks() {}

function $milo_enum_MessageTypes(MESSAGE_TYPE_) {}
function $milo_enum_Errors(ERROR_) {}
function $milo_enum_Methods(METHOD_) {}
function $milo_enum_Connections(CONNECTION_) {}
function $milo_enum_Callbacks(CALLBACK_) {}
function $milo_enum_States(STATE_) {}

// Loader
function load() {
  const state = []

  // Avoid hidden classes
  const callbackContext = { memory: null, alloc: null, dealloc: null, fail: null, state }

  const bytes = readFileSync(resolve(__dirname, 'milo.wasm'))
  const mod = new WebAssembly.Module(bytes)
  const instance = new WebAssembly.Instance(mod, {
    env: { run_callback: runCallback.bind(null, callbackContext), logger }
  })
  const wasm = instance.exports

  callbackContext.fail = fail.bind(wasm)

  const milo = {
    $milo_version,
    memory: wasm.memory,
    state,
    alloc: alloc.bind(wasm),
    create: create.bind(wasm, state),
    destroy: destroy.bind(wasm, state),
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
    getCallbackError: getCallbackError.bind(wasm, state),
    // eslint-disable-next-line no-dupe-keys
    $milo_wasm: {
      setContinueWithoutData,
      setIsConnect,
      setManageUnconsumed,
      setMode,
      setSkipBody
    },
    $milo_callbacks,
    $milo_enums,
    $milo_constants,
    FLAG_DEBUG: $milo_flag_debug
  }

  callbackContext.memory = wasm.memory
  callbackContext.alloc = milo.alloc
  callbackContext.dealloc = milo.dealloc
  callbackContext.fail = milo.fail

  return milo
}

module.exports = { load, milo: load() }
