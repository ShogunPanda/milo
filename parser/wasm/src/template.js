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

function destroy (parser) {
  this.destroy(parser)
}

function parse (parser, data, limit) {
  return this.parse(parser, data, limit) >>> 0
}

function fail (parser, code, description) {
  const len = description.length
  const ptr = this.alloc(len)
  const buffer = new Uint8Array(this.memory.buffer, ptr, len)
  textEncoder.encodeInto(description, buffer)

  this.fail(parser, code, ptr, len)
  this.dealloc(ptr, len)
}

function hasDebug () {
  return this.milo_has_debug() !== 0
}

/* REPLACE: enums */

/* REPLACE: getters */

/* REPLACE: setters */

// The postbuild step injects generated callback references.
// eslint-disable-next-line no-unused-vars
function collectSpan (spans, spanOverflow, maxSpans, parser, span) {
  if (spans[parser].length < maxSpans) {
    spans[parser].push(span)
  } else {
    spanOverflow[parser] = true
  }
}

function simpleCreate (spans, spanOverflow, create) {
  const parser = create()
  spans[parser] = []
  spanOverflow[parser] = false
  this.setActiveCallbacks(parser, this.CALLBACK_ACTIVE_ALL)
  return parser
}

function simpleDestroy (spans, spanOverflow, destroy, parser) {
  delete spans[parser]
  delete spanOverflow[parser]
  destroy(parser)
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
  let { logger: logOption, ...instanceEnvironment } = env
  let logger = noop
  const context = {}

  if (logOption) {
    if (typeof logOption !== 'function') {
      logOption = console.log
    }

    logger = log.bind(context, logOption)
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
    destroy: destroy.bind(wasm),
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
  const milo = setup({
    /* REPLACE: callbacks:simple */
  })

  milo.spans = spans
  milo.spanOverflow = spanOverflow
  milo.maxSpans = maxSpans
  milo.create = simpleCreate.bind(milo, spans, spanOverflow, milo.create)
  milo.destroy = simpleDestroy.bind(milo, spans, spanOverflow, milo.destroy)
  milo.drain = simpleDrain.bind(undefined, spans, spanOverflow)

  return milo
}
