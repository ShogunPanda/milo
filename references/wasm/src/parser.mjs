import { isMainThread } from 'node:worker_threads'

export const info = isMainThread ? console.log : () => {}

function extractPayload(context, from, size) {
  return context.input.subarray(from, from + size)
}

function getOffsets(context) {
  const start = context.milo.getOffsets(context.parser)
  const flags = new Uint32Array(context.milo.memory.buffer, start, 3)
  const total = flags[2]
  flags[2] = 0
  return new Uint32Array(context.milo.memory.buffer, start + 12, total * 3)
}

function sprintf(format, ...args) {
  return format.replaceAll('{}', () => args.shift())
}

function formatEvent(name) {
  return `"${name}"`
}

function appendOutput(message, context, from, size) {
  const payload = typeof from === 'number' ? `"${extractPayload(context, from, size).toString('utf-8')}"` : 'null'
  info(`{ ${message}, "data": ${payload} }`)
  return 0
}

function event(parser, name, position, from, size) {
  return appendOutput(sprintf('"pos": {}, "event": "{}"', position, name), parser, from, size)
}

function showSpan(context, name, from, size) {
  if (name == 'method' || name == 'url' || name == 'protocol' || name == 'version') {
    context[name] = extractPayload(context, from, size).toString('utf-8')
  }

  return event(context, name, context.milo.getPosition(context.parser), from, size)
}

function beforeStateChange(context, from, size) {
  return appendOutput(
    sprintf(
      '"pos": {}, "event": "before_state_change", "current_state": "{}"',
      this.getPosition(context.parser),
      this.States[this.getState(context.parser)]
    ),
    context,
    from,
    size
  )
}

function afterStateChange(context, from, size) {
  return appendOutput(
    sprintf(
      '"pos": {}, "event": "after_state_change", "current_state": "{}"',
      this.getPosition(context.parser),
      this.States[this.getState(context.parser)]
    ),
    context,
    from,
    size
  )
}

function onMessageStart(context, from, size) {
  return appendOutput(
    sprintf(
      '"pos": {}, "event": "begin", "configuration": { "debug": {} }',
      this.getPosition(context.parser),
      this.FLAGS_DEBUG
    ),
    context,
    from,
    size
  )
}

function onMessageComplete(context, from, size) {
  return event(context, 'complete', this.getPosition(context.parser), from, size)
}

function onError(context, from, size) {
  const errorDescription = Buffer.from(
    this.memory.buffer,
    this.getErrorDescription(context.parser),
    this.getErrorDescriptionLen(context.parser)
  )

  let callbackError = this.getCallbackError(context.parser)

  if (callbackError) {
    callbackError = JSON.stringify({
      type: callbackError.name,
      message: callbackError.message,
      stack: callbackError.stack
    })
  }

  return appendOutput(
    sprintf(
      '"pos": {}, "event": {}, "error_code": {}, "error_code_string": "{}", reason: "{}", callbackError: {}',
      this.getPosition(context.parser),
      'error',
      this.getErrorCode(context.parser),
      this.Errors[this.getErrorCode(context.parser)],
      errorDescription.toString(),
      callbackError
    ),
    context,
    from,
    size
  )
}

function onFinish(context, from, size) {
  return event(context, 'finish', this.getPosition(context.parser), from, size)
}

function onRequest(context, from, size) {
  return event(context, 'request', this.getPosition(context.parser), from, size)
}

function onResponse(context, from, size) {
  return event(context, 'response', this.getPosition(context.parser), from, size)
}

function onMethod(context, from, size) {
  return showSpan(context, 'method', from, size)
}

function onUrl(context, from, size) {
  return showSpan(context, 'url', from, size)
}

function onProtocol(context, from, size) {
  return showSpan(context, 'protocol', from, size)
}

function onVersion(context, from, size) {
  return showSpan(context, 'version', from, size)
}

function onStatus(context, from, size) {
  return showSpan(context, 'status', from, size)
}

function onReason(context, from, size) {
  return showSpan(context, 'reason', from, size)
}

function onHeaderName(context, from, size) {
  return showSpan(context, 'header_name', from, size)
}

function onHeaderValue(context, from, size) {
  return showSpan(context, 'header_value', from, size)
}

function onHeaders(context, from, size) {
  const position = this.getPosition(context.parser)
  const chunked = this.hasChunkedTransferEncoding(context.parser)
  const content_length = this.getContentLength(context.parser)
  let method = context.method
  let url = context.url
  let protocol = context.protocol
  let version = context.version

  const offsets = getOffsets(context)

  for (let i = 0; i < offsets.length / 3; i++) {
    const offsetType = offsets[i * 3]
    const offsetFrom = offsets[i * 3 + 1]
    const offsetSize = offsets[i * 3 + 2]

    switch (offsetType) {
      case this.Offsets.METHOD:
        event(context, 'offset.method', offsetFrom, offsetFrom, offsetSize)
        method = extractPayload(context, offsetFrom, offsetSize)
        break
      case this.Offsets.URL:
        event(context, 'offset.url', offsetFrom, offsetFrom, offsetSize)
        url = extractPayload(context, offsetFrom, offsetSize)
        break
      case this.Offsets.PROTOCOL:
        event(context, 'offset.protocol', offsetFrom, offsetFrom, offsetSize)
        protocol = extractPayload(context, offsetFrom, offsetSize)
        break
      case this.Offsets.VERSION:
        event(context, 'offset.version', offsetFrom, offsetFrom, offsetSize)
        version = extractPayload(context, offsetFrom, offsetSize)
        break
      case this.Offsets.STATUS:
        event(context, 'offset.status', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.REASON:
        event(context, 'offset.reason', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.HEADER_NAME:
        event(context, 'offset.header_name', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.HEADER_VALUE:
        event(context, 'offset.header_value', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.CHUNK_LENGTH:
        event(context, 'offset.chunk_length', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.CHUNK_EXTENSION_NAME:
        event(context, 'offset.chunk_extensions_name', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.CHUNK_EXTENSION_VALUE:
        event(context, 'offset.chunk_extension_value', offsetFrom, offsetFrom, offsetSize)
        break
      default:
        throw new Error('Unexpected offset with type ', offsetType)
    }
  }

  if (this.getMessageType(context.parser) == this.RESPONSE) {
    const heading = sprintf('"pos": {}, "event": {}, "type": "response", ', position, formatEvent('headers'))

    if (chunked) {
      return appendOutput(
        sprintf(
          '{}"status": {}, "protocol": "{}", "version": "{}", "body": "chunked"',
          heading,
          this.getStatus(context.parser),
          protocol,
          version
        ),
        context,
        from,
        size
      )
    } else if (content_length > 0) {
      return appendOutput(
        sprintf(
          '{}"status": {}, "protocol": "{}", "version": "{}", "body": {}"',
          heading,
          this.getStatus(context.parser),
          protocol,
          version,
          content_length
        ),
        context,
        from,
        size
      )
    } else {
      return appendOutput(
        sprintf(
          '{}"status": {}, "protocol": "{}", "version": "{}", "body": null',
          heading,
          this.getStatus(context.parser),
          protocol,
          version
        ),
        context,
        from,
        size
      )
    }
  } else {
    const heading = sprintf('"pos": {}, "event": {}, "type": "request", ', position, formatEvent('headers'))

    if (chunked) {
      return appendOutput(
        sprintf(
          '{}"method": "{}", "url": "{}", "protocol": "{}", "version": "{}", "body": "chunked"',
          heading,
          method,
          url,
          protocol,
          version
        ),
        context,
        from,
        size
      )
    } else if (content_length > 0) {
      return appendOutput(
        sprintf(
          '{}"method": "{}", "url": "{}", "protocol": "{}", "version": "{}", "body": {}',
          heading,
          method,
          url,
          protocol,
          version,
          content_length
        ),
        context,
        from,
        size
      )
    } else {
      return appendOutput(
        sprintf(
          '{}"method": "{}", "url": "{}", "protocol": "{}", "version": "{}", "body": null',
          heading,
          method,
          url,
          protocol,
          version
        ),
        context,
        from,
        size
      )
    }
  }
}

function onUpgrade(context, from, size) {
  return event(context, 'upgrade', this.getPosition(context.parser), from, size)
}

function onChunkLength(context, from, size) {
  return showSpan(context, 'chunk_length', from, size)
}

function onChunkExtensionName(context, from, size) {
  return showSpan(context, 'chunk_extensions_name', from, size)
}

function onChunkExtensionValue(context, from, size) {
  return showSpan(context, 'chunk_extension_value', from, size)
}

function onChunk(context, from, size) {
  const offsets = getOffsets(context)

  for (let i = 0; i < offsets.length / 3; i++) {
    const offsetType = offsets[i * 3]
    const offsetFrom = offsets[i * 3 + 1]
    const offsetSize = offsets[i * 3 + 2]

    switch (offsetType) {
      case this.Offsets.CHUNK_LENGTH:
        event(context, 'offset.chunk_length', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.CHUNK_EXTENSION_NAME:
        event(context, 'offset.chunk_extensions_name', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.CHUNK_EXTENSION_VALUE:
        event(context, 'offset.chunk_extension_value', offsetFrom, offsetFrom, offsetSize)
        break
    }
  }

  return event(context, 'chunk', this.getPosition(context.parser), from, size)
}

function onBody(context, from, size) {
  return event(context, 'body', this.getPosition(context.parser), from, size)
}

function onData(context, from, size) {
  return showSpan(context, 'data', from, size)
}

function onTrailerName(context, from, size) {
  return showSpan(context, 'trailer_name', from, size)
}

function onTrailerValue(context, from, size) {
  return showSpan(context, 'trailer_value', from, size)
}

function onTrailers(context, from, size) {
  const offsets = getOffsets(context)

  for (let i = 0; i < offsets.length / 3; i++) {
    const offsetType = offsets[i * 3]
    const offsetFrom = offsets[i * 3 + 1]
    const offsetSize = offsets[i * 3 + 2]

    switch (offsets[i * 3]) {
      case this.Offsets.TRAILER_NAME:
        event(context, 'offset.trailer_name', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.TRAILER_VALUE:
        event(context, 'offset.trailer_value', offsetFrom, offsetFrom, offsetSize)
        break
    }
  }

  return event(context, 'trailers', this.getPosition(context.parser), from, size)
}

let testData = undefined
export async function load() {
  if (testData) {
    return testData
  }

  const milo = await import(`../lib/${process.env.CONFIGURATION ?? process.argv[2]}/milo.js`)
  const parser = milo.create()
  const context = { milo, parser }

  milo.setBeforeStateChange(parser, beforeStateChange.bind(milo, context))
  milo.setAfterStateChange(parser, afterStateChange.bind(milo, context))
  milo.setOnError(parser, onError.bind(milo, context))
  milo.setOnFinish(parser, onFinish.bind(milo, context))
  milo.setOnRequest(parser, onRequest.bind(milo, context))
  milo.setOnResponse(parser, onResponse.bind(milo, context))
  milo.setOnMessageStart(parser, onMessageStart.bind(milo, context))
  milo.setOnMessageComplete(parser, onMessageComplete.bind(milo, context))
  milo.setOnMethod(parser, onMethod.bind(milo, context))
  milo.setOnUrl(parser, onUrl.bind(milo, context))
  milo.setOnProtocol(parser, onProtocol.bind(milo, context))
  milo.setOnVersion(parser, onVersion.bind(milo, context))
  milo.setOnStatus(parser, onStatus.bind(milo, context))
  milo.setOnReason(parser, onReason.bind(milo, context))
  milo.setOnHeaderName(parser, onHeaderName.bind(milo, context))
  milo.setOnHeaderValue(parser, onHeaderValue.bind(milo, context))
  milo.setOnHeaders(parser, onHeaders.bind(milo, context))
  milo.setOnUpgrade(parser, onUpgrade.bind(milo, context))
  milo.setOnChunkLength(parser, onChunkLength.bind(milo, context))
  milo.setOnChunkExtensionName(parser, onChunkExtensionName.bind(milo, context))
  milo.setOnChunkExtensionValue(parser, onChunkExtensionValue.bind(milo, context))
  milo.setOnChunk(parser, onChunk.bind(milo, context))
  milo.setOnBody(parser, onBody.bind(milo, context))
  milo.setOnData(parser, onData.bind(milo, context))
  milo.setOnTrailerName(parser, onTrailerName.bind(milo, context))
  milo.setOnTrailerValue(parser, onTrailerValue.bind(milo, context))
  milo.setOnTrailers(parser, onTrailers.bind(milo, context))

  const request1 = Buffer.from('GET / HTTP/1.1\r\n\r\n')
  const request2 = Buffer.from(
    'HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: x-trailer\r\n\r\nc;need=love\r\nhello world!\r\n0\r\nX-Trailer: value\r\n\r\n'
  )
  const request3 = Buffer.from(
    'HTTP/1.1 200 OK\r\nDate: Wed, 15 Nov 2023 21:06:00 GMT\r\nConnection: keep-alive\r\nKeep-Alive: timeout=600\r\nContent-Length: 65536\r\n\r\n' +
      Buffer.alloc(64 * 1024, '_').toString()
  )

  testData = [milo, parser, context, request1, request2, request3]
  return testData
}
