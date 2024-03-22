import { isMainThread } from 'node:worker_threads'

export const info = isMainThread ? console.log : () => {}

function extractPayload(context, from, size) {
  return context.input.subarray(from, from + size)
}

function processOffsets(context) {
  const start = context.milo.getOffsets(context.parser)
  const flags = new Uint32Array(context.milo.memory.buffer, start, 3)
  const total = context.milo.getOffsetsCount(context.parser)
  context.milo.clearOffsets(context.parser)

  const offsets = new Uint32Array(context.milo.memory.buffer, start, total * 3)

  for (let i = 0; i < total; i++) {
    const offsetType = offsets[i * 3]
    const offsetFrom = offsets[i * 3 + 1]
    const offsetSize = offsets[i * 3 + 2]

    switch (offsetType) {
      case context.milo.Offsets.MESSAGE_START:
        event(context, 'offset.message_start', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.MESSAGE_COMPLETE:
        event(context, 'offset.message_complete', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.METHOD:
        event(context, 'offset.method', offsetFrom, offsetFrom, offsetSize)
        context.method = extractPayload(context, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.URL:
        event(context, 'offset.url', offsetFrom, offsetFrom, offsetSize)
        context.url = extractPayload(context, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.PROTOCOL:
        event(context, 'offset.protocol', offsetFrom, offsetFrom, offsetSize)
        context.protocol = extractPayload(context, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.VERSION:
        event(context, 'offset.version', offsetFrom, offsetFrom, offsetSize)
        context.version = extractPayload(context, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.STATUS:
        event(context, 'offset.status', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.REASON:
        event(context, 'offset.reason', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.HEADER_NAME:
        event(context, 'offset.header_name', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.HEADER_VALUE:
        event(context, 'offset.header_value', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.HEADERS:
        event(context, 'offset.headers', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.CHUNK_LENGTH:
        event(context, 'offset.chunk_length', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.CHUNK_EXTENSION_NAME:
        event(context, 'offset.chunk_extensions_name', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.CHUNK_EXTENSION_VALUE:
        event(context, 'offset.chunk_extension_value', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.CHUNK:
        event(context, 'offset.chunk', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.DATA:
        event(context, 'offset.data', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.BODY:
        event(context, 'offset.body', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.TRAILER_NAME:
        event(context, 'offset.trailer_name', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.TRAILER_VALUE:
        event(context, 'offset.trailer_value', offsetFrom, offsetFrom, offsetSize)
        break
      case context.milo.Offsets.TRAILERS:
        event(context, 'offset.trailers', offsetFrom, offsetFrom, offsetSize)
        break
      default:
        throw new Error('Unexpected offset with type ', offsetType)
    }
  }
}

function sprintf(format, ...args) {
  return format.replaceAll('{}', () => args.shift())
}

function formatEvent(name) {
  return `"${name}"`
}

function appendOutput(message, context, from, size) {
  const payload =
    typeof from === 'number' && typeof size === 'number' && size > 0
      ? `"${extractPayload(context, from, size).toString('utf-8')}"`
      : 'null'
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

  return event(context, name, context.values.readUInt32LE(context.milo.VALUES_POSITION), from, size)
}

function beforeStateChange(context, from, size) {
  return appendOutput(
    sprintf(
      '"pos": {}, "event": "before_state_change", "current_state": "{}"',
      context.values.readUInt32LE(context.milo.VALUES_POSITION),
      context.milo.States[context.values.readUInt32LE(context.milo.VALUES_STATE)]
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
      context.values.readUInt32LE(context.milo.VALUES_POSITION),
      context.milo.States[context.values.readUInt32LE(context.milo.VALUES_STATE)]
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
      context.values.readUInt32LE(context.milo.VALUES_POSITION),
      this.FLAGS_DEBUG
    ),
    context,
    from,
    size
  )
}

function onMessageComplete(context, from, size) {
  processOffsets(context)
  return event(context, 'complete', context.values.readUInt32LE(context.milo.VALUES_POSITION), from, size)
}

function onError(context, from, size) {
  const errorDescription = Buffer.from(
    this.memory.buffer,
    this.getErrorDescription(context.parser),
    context.values.readUInt32LE(context.milo.ERROR_DESCRIPTION_LEN)
  )

  let callbackError = this.getCallbackError(context.parser)

  if (callbackError) {
    callbackError = JSON.stringify({
      type: callbackError.name,
      message: callbackError.message,
      stack: callbackError.stack
    })
  }

  const errorCode = context.values.readUInt32LE(context.milo.VALUES_ERROR_CODE)
  return appendOutput(
    sprintf(
      '"pos": {}, "event": {}, "error_code": {}, "error_code_string": "{}", reason: "{}", callbackError: {}',
      context.values.readUInt32LE(context.milo.VALUES_POSITION),
      'error',
      errorCode,
      this.Errors[errorCode],
      errorDescription.toString(),
      callbackError
    ),
    context,
    from,
    size
  )
}

function onFinish(context, from, size) {
  return event(context, 'finish', context.values.readUInt32LE(context.milo.VALUES_POSITION), from, size)
}

function onRequest(context, from, size) {
  return event(context, 'request', context.values.readUInt32LE(context.milo.VALUES_POSITION), from, size)
}

function onResponse(context, from, size) {
  return event(context, 'response', context.values.readUInt32LE(context.milo.VALUES_POSITION), from, size)
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
  processOffsets(context)

  const position = context.values.readUInt32LE(context.milo.VALUES_POSITION)
  const chunked = context.values.readUInt32LE(context.milo.VALUES_HAS_CHUNKED_TRANSFER_ENCODING)
  const content_length = context.values.readUInt32LE(context.milo.VALUES_CONTENT_LENGTH)
  let method = context.method
  let url = context.url
  let protocol = context.protocol
  let version = context.version

  if (context.values.readUInt32LE(context.milo.VALUES_MESSAGE_TYPE) == context.milo.RESPONSE) {
    const status = context.values.readUInt32LE(context.milo.VALUES_STATUS)
    const heading = sprintf('"pos": {}, "event": {}, "type": "response", ', position, formatEvent('headers'))

    if (chunked) {
      return appendOutput(
        sprintf(
          '{}"status": {}, "protocol": "{}", "version": "{}", "body": "chunked"',
          heading,
          status,
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
          status,
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
        sprintf('{}"status": {}, "protocol": "{}", "version": "{}", "body": null', heading, status, protocol, version),
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
  return event(context, 'upgrade', context.values.readUInt32LE(context.milo.VALUES_POSITION), from, size)
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
  processOffsets(context)
  return event(context, 'chunk', context.values.readUInt32LE(context.milo.VALUES_POSITION), from, size)
}

function onBody(context, from, size) {
  return event(context, 'body', context.values.readUInt32LE(context.milo.VALUES_POSITION), from, size)
}

function onData(context, from, size) {
  processOffsets(context)
  return showSpan(context, 'data', from, size)
}

function onTrailerName(context, from, size) {
  return showSpan(context, 'trailer_name', from, size)
}

function onTrailerValue(context, from, size) {
  return showSpan(context, 'trailer_value', from, size)
}

function onTrailers(context, from, size) {
  processOffsets(context)
  return event(context, 'trailers', context.values.readUInt32LE(context.milo.VALUES_POSITION), from, size)
}

let testData = undefined
export async function load() {
  if (testData) {
    return testData
  }

  const milo = await import(`../lib/${process.env.CONFIGURATION ?? process.argv[2]}/milo.js`)
  const parser = milo.create()
  const context = {
    milo,
    parser,
    values: Buffer.from(milo.memory.buffer, milo.getValues(parser), milo.VALUES_SIZE),
    offsets: Buffer.from(milo.memory.buffer, milo.getOffsets(parser), milo.MAX_OFFSETS_COUNT)
  }

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
