/* REPLACE: module */

const textEncoder = new TextEncoder()
const textDecoder = new TextDecoder()

function log (logger, raw) {
  const len = Number(BigInt.asUintN(32, raw))
  const ptr = Number(raw >> 32n)

  logger(textDecoder.decode(new Uint8Array(this.memory.buffer, ptr, len)))
}

function alloc (len) {
  return this.alloc(len) >>> 0
}

function dealloc (ptr, len) {
  return this.dealloc(ptr, len)
}

function create () {
  return this.create() >>> 0
}

function destroy (onDestroy, parser) {
  if (this.destroy(parser) && onDestroy) {
    onDestroy(parser)
  }
}

function parse (parser, data, limit) {
  return this.parse(parser, data, limit) >>> 0
}

function fail (parser, code, description) {
  const bytes = textEncoder.encode(description)
  const len = bytes.byteLength
  const ptr = this.alloc(len)

  try {
    new Uint8Array(this.memory.buffer, ptr, len).set(bytes)
    this.fail(parser, code, ptr, len)
  } finally {
    this.dealloc(ptr, len)
  }
}

function hasDebug () {
  return this.milo_has_debug() !== 0
}

function wrapOnHeaders (callback) {
  return (parser, at, methodOrStatus, keepAlive, shouldUpgrade, hasTrailers, bodyKind, contentLength) => {
    callback(
      parser,
      at,
      methodOrStatus,
      keepAlive !== 0,
      shouldUpgrade !== 0,
      hasTrailers !== 0,
      bodyKind,
      BigInt.asUintN(64, contentLength)
    )
  }
}

/* REPLACE: enums */

/* REPLACE: getters */

/* REPLACE: setters */

// The postbuild step injects generated callback references.
// eslint-disable-next-line no-unused-vars
function collectSpan (spans, spanOverflow, maxSpans, parser, type, at, len) {
  const batch = spans[parser]
  if (batch.length >= maxSpans) {
    spanOverflow[parser] = true
    return
  }

  batch.push([type, at, len])
}

function simpleCreate (spans, spanOverflow, create) {
  const parser = create()
  spans[parser] = []
  spanOverflow[parser] = false
  this.setActiveCallbacks(parser, this.CALLBACK_ACTIVE_ALL)
  return parser
}

function clearSpans (spans, spanOverflow, parser) {
  delete spans[parser]
  delete spanOverflow[parser]
}

function simpleDrain (spans, spanOverflow, parser) {
  const drained = spans[parser] ?? []
  const truncated = spanOverflow[parser] ?? false

  if (spans[parser] !== undefined) {
    spans[parser] = []
    spanOverflow[parser] = false
  }

  return { spans: drained, truncated }
}

export function noop () {}

export function setup (env = {}) {
  return setupInstance(env)
}

function setupInstance (env, onDestroy) {
  let { logger: logOption, ...instanceEnvironment } = env
  let logger = noop
  const context = {}

  if (logOption) {
    if (typeof logOption !== 'function') {
      logOption = console.log
    }

    logger = log.bind(context, logOption)
  }

  if (instanceEnvironment.on_headers) {
    instanceEnvironment.on_headers = wrapOnHeaders(instanceEnvironment.on_headers)
  }

  // Create the WASM instance
  /* eslint-disable-next-line no-undef */
  const instance = new WebAssembly.Instance(wasmModule, {
    env: {
      logger,
      /* REPLACE: callbacks:noop */
      ...instanceEnvironment
    }
  })

  const wasm = instance.exports
  context.memory = wasm.memory

  const milo = {
    /* REPLACE: version */
    /* REPLACE: constants */
    /* REPLACE: enums:list */
    /* REPLACE: getters:list */
    /* REPLACE: setters:list */
    memory: wasm.memory,
    alloc: alloc.bind(wasm),
    dealloc: dealloc.bind(wasm),
    create: create.bind(wasm),
    destroy: destroy.bind(wasm, onDestroy),
    parse: parse.bind(wasm),
    fail: fail.bind(wasm),
    hasDebug: hasDebug.bind(wasm),
    clear: wasm.clear,
    complete: wasm.complete,
    finish: wasm.finish,
    pause: wasm.pause,
    reset: wasm.reset,
    resume: wasm.resume
  }

  /* REPLACE: start */
  return milo
}

export function simple ({ maxSpans = 65536 } = {}) {
  if (!Number.isSafeInteger(maxSpans) || maxSpans < 0) {
    throw new RangeError('maxSpans must be a non-negative safe integer')
  }

  const spans = Object.create(null)
  const spanOverflow = Object.create(null)

  /* prettier-ignore */
  const env = {
    /* REPLACE: callbacks:simple */
  }
  const milo = setupInstance(env, clearSpans.bind(undefined, spans, spanOverflow))

  milo.spans = spans
  milo.spanOverflow = spanOverflow
  milo.maxSpans = maxSpans
  milo.create = simpleCreate.bind(milo, spans, spanOverflow, milo.create)
  milo.drain = simpleDrain.bind(undefined, spans, spanOverflow)

  return milo
}
