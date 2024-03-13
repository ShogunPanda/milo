import { isMainThread } from 'node:worker_threads'

export const info = isMainThread ? console.log : () => {}

function extractPayload(parser, from, size) {
  return parser.context.input.subarray(from, from + size)
}

function sprintf(format, ...args) {
  return format.replaceAll('{}', () => args.shift())
}

function formatEvent(name) {
  return `"${name}"`
}

function appendOutput(message, parser, from, size) {
  const payload = typeof from === 'number' ? `"${extractPayload(parser, from, size).toString('utf-8')}"` : 'null'
  info(`{ ${message}, "data": ${payload} }`)
  return 0
}

function event(parser, name, position, from, size) {
  return appendOutput(sprintf('"pos": {}, "event": "{}"', position, name), parser, from, size)
}

function showSpan(parser, name, from, size) {
  if (name == 'method' || name == 'url' || name == 'protocol' || name == 'version') {
    parser.context[name] = extractPayload(parser, from, size).toString('utf-8')
  }

  return event(parser, name, parser.position, from, size)
}

function beforeStateChange(parser, from, size) {
  return appendOutput(
    sprintf(
      '"pos": {}, "event": "before_state_change", "current_state": "{}"',
      parser.position,
      this.States[parser.state]
    ),
    parser,
    from,
    size
  )
}

function afterStateChange(parser, from, size) {
  return appendOutput(
    sprintf(
      '"pos": {}, "event": "after_state_change", "current_state": "{}"',
      parser.position,
      this.States[parser.state]
    ),
    parser,
    from,
    size
  )
}

function onMessageStart(parser, from, size) {
  return appendOutput(
    sprintf('"pos": {}, "event": "begin", "configuration": { "debug": {} }', parser.position, this.FLAGS_DEBUG),
    parser,
    from,
    size
  )
}

function onMessageComplete(parser, from, size) {
  return event(parser, 'complete', parser.position, from, size)
}

function onError(parser, from, size) {
  return appendOutput(
    sprintf(
      '"pos": {}, "event": {}, "error_code": {}, "error_code_string": "{}", reason: "{}"',
      parser.position,
      'error',
      parser.errorCode,
      this.Errors[parser.errorCode],
      parser.errorDescription
    ),
    parser,
    from,
    size
  )
}

function onFinish(parser, from, size) {
  return event(parser, 'finish', parser.position, from, size)
}

function onRequest(parser, from, size) {
  return event(parser, 'request', parser.position, from, size)
}

function onResponse(parser, from, size) {
  return event(parser, 'response', parser.position, from, size)
}

function onMethod(parser, from, size) {
  return showSpan(parser, 'method', from, size)
}

function onUrl(parser, from, size) {
  return showSpan(parser, 'url', from, size)
}

function onProtocol(parser, from, size) {
  return showSpan(parser, 'protocol', from, size)
}

function onVersion(parser, from, size) {
  return showSpan(parser, 'version', from, size)
}

function onStatus(parser, from, size) {
  return showSpan(parser, 'status', from, size)
}

function onReason(parser, from, size) {
  return showSpan(parser, 'reason', from, size)
}

function onHeaderName(parser, from, size) {
  return showSpan(parser, 'header_name', from, size)
}

function onHeaderValue(parser, from, size) {
  return showSpan(parser, 'header_value', from, size)
}

function onHeaders(parser, from, size) {
  const position = parser.position
  const chunked = parser.hasChunkedTransferEncoding
  const content_length = parser.contentLength
  let method = parser.context.method
  let url = parser.context.url
  let protocol = parser.context.protocol
  let version = parser.context.version

  const offsets = parser.offsets
  const total = offsets[2]

  for (let i = 1; i <= total; i++) {
    const [offsetType, offsetFrom, offsetSize] = offsets.slice(i * 3, i * 3 + 3)

    switch (offsetType) {
      case this.Offsets.METHOD:
        event(parser, 'offset.method', offsetFrom, offsetFrom, offsetSize)
        method = extractPayload(parser, offsetFrom, offsetSize)
        break
      case this.Offsets.URL:
        event(parser, 'offset.url', offsetFrom, offsetFrom, offsetSize)
        url = extractPayload(parser, offsetFrom, offsetSize)
        break
      case this.Offsets.PROTOCOL:
        event(parser, 'offset.protocol', offsetFrom, offsetFrom, offsetSize)
        protocol = extractPayload(parser, offsetFrom, offsetSize)
        break
      case this.Offsets.VERSION:
        event(parser, 'offset.version', offsetFrom, offsetFrom, offsetSize)
        version = extractPayload(parser, offsetFrom, offsetSize)
        break
      case this.Offsets.STATUS:
        event(parser, 'offset.status', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.REASON:
        event(parser, 'offset.reason', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.HEADER_NAME:
        event(parser, 'offset.header_name', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.HEADER_VALUE:
        event(parser, 'offset.header_value', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.CHUNK_LENGTH:
        event(parser, 'offset.chunk_length', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.CHUNK_EXTENSION_NAME:
        event(parser, 'offset.chunk_extensions_name', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.CHUNK_EXTENSION_VALUE:
        event(parser, 'offset.chunk_extension_value', offsetFrom, offsetFrom, offsetSize)
        break
      default:
        throw new Error('Unexpected offset with type ', +offsets[i * 3])
    }
  }

  offsets[2] = 0

  if (parser.messageType == this.RESPONSE) {
    const heading = sprintf('"pos": {}, "event": {}, "type": "response", ', position, formatEvent('headers'))

    if (chunked) {
      return appendOutput(
        sprintf(
          '{}"status": {}, "protocol": "{}", "version": "{}", "body": "chunked"',
          heading,
          parser.status,
          protocol,
          version
        ),
        parser,
        from,
        size
      )
    } else if (content_length > 0) {
      return appendOutput(
        sprintf(
          '{}"status": {}, "protocol": "{}", "version": "{}", "body": {}"',
          heading,
          parser.status,
          protocol,
          version,
          content_length
        ),
        parser,
        from,
        size
      )
    } else {
      return appendOutput(
        sprintf(
          '{}"status": {}, "protocol": "{}", "version": "{}", "body": null',
          heading,
          parser.status,
          protocol,
          version
        ),
        parser,
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
        parser,
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
        parser,
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
        parser,
        from,
        size
      )
    }
  }
}

function onUpgrade(parser, from, size) {
  return event(parser, 'upgrade', parser.position, from, size)
}

function onChunkLength(parser, from, size) {
  return showSpan(parser, 'chunk_length', from, size)
}

function onChunkExtensionName(parser, from, size) {
  return showSpan(parser, 'chunk_extensions_name', from, size)
}

function onChunkExtensionValue(parser, from, size) {
  return showSpan(parser, 'chunk_extension_value', from, size)
}

function onChunk(parser, from, size) {
  const offsets = parser.offsets
  const total = offsets[2]

  for (let i = 1; i <= total; i++) {
    const [offsetType, offsetFrom, offsetSize] = offsets.slice(i * 3, i * 3 + 3)

    switch (offsetType) {
      case this.Offsets.CHUNK_LENGTH:
        event(parser, 'offset.chunk_length', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.CHUNK_EXTENSION_NAME:
        event(parser, 'offset.chunk_extensions_name', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.CHUNK_EXTENSION_VALUE:
        event(parser, 'offset.chunk_extension_value', offsetFrom, offsetFrom, offsetSize)
        break
    }
  }

  offsets[2] = 0

  return event(parser, 'chunk', parser.position, from, size)
}

function onBody(parser, from, size) {
  return event(parser, 'body', parser.position, from, size)
}

function onData(parser, from, size) {
  return showSpan(parser, 'data', from, size)
}

function onTrailerName(parser, from, size) {
  return showSpan(parser, 'trailer_name', from, size)
}

function onTrailerValue(parser, from, size) {
  return showSpan(parser, 'trailer_value', from, size)
}

function onTrailers(parser, from, size) {
  const offsets = parser.offsets
  const total = offsets[2]

  for (let i = 1; i <= total; i++) {
    const [offsetType, offsetFrom, offsetSize] = offsets.slice(i * 3, i * 3 + 3)

    switch (offsets[i * 3]) {
      case this.Offsets.TRAILER_NAME:
        event(parser, 'offset.trailer_name', offsetFrom, offsetFrom, offsetSize)
        break
      case this.Offsets.TRAILER_VALUE:
        event(parser, 'offset.trailer_value', offsetFrom, offsetFrom, offsetSize)
        break
    }
  }

  offsets[2] = 0

  return event(parser, 'trailers', parser.position, from, size)
}

let testData = undefined
export async function load() {
  if (testData) {
    return testData
  }

  const milo = await import(`../lib/${process.env.CONFIGURATION ?? process.argv[2]}/milo.js`)
  const parser = milo.Parser.create()

  parser.setBeforeStateChange(beforeStateChange.bind(milo, parser))
  parser.setAfterStateChange(afterStateChange.bind(milo, parser))
  parser.setOnError(onError.bind(milo, parser))
  parser.setOnFinish(onFinish.bind(milo, parser))
  parser.setOnRequest(onRequest.bind(milo, parser))
  parser.setOnResponse(onResponse.bind(milo, parser))
  parser.setOnMessageStart(onMessageStart.bind(milo, parser))
  parser.setOnMessageComplete(onMessageComplete.bind(milo, parser))
  parser.setOnMethod(onMethod.bind(milo, parser))
  parser.setOnUrl(onUrl.bind(milo, parser))
  parser.setOnProtocol(onProtocol.bind(milo, parser))
  parser.setOnVersion(onVersion.bind(milo, parser))
  parser.setOnStatus(onStatus.bind(milo, parser))
  parser.setOnReason(onReason.bind(milo, parser))
  parser.setOnHeaderName(onHeaderName.bind(milo, parser))
  parser.setOnHeaderValue(onHeaderValue.bind(milo, parser))
  parser.setOnHeaders(onHeaders.bind(milo, parser))
  parser.setOnUpgrade(onUpgrade.bind(milo, parser))
  parser.setOnChunkLength(onChunkLength.bind(milo, parser))
  parser.setOnChunkExtensionName(onChunkExtensionName.bind(milo, parser))
  parser.setOnChunkExtensionValue(onChunkExtensionValue.bind(milo, parser))
  parser.setOnChunk(onChunk.bind(milo, parser))
  parser.setOnBody(onBody.bind(milo, parser))
  parser.setOnData(onData.bind(milo, parser))
  parser.setOnTrailerName(onTrailerName.bind(milo, parser))
  parser.setOnTrailerValue(onTrailerValue.bind(milo, parser))
  parser.setOnTrailers(onTrailers.bind(milo, parser))

  const request1 = Buffer.from('GET / HTTP/1.1\r\n\r\n')
  const request2 = Buffer.from(
    'HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: x-trailer\r\n\r\nc;need=love\r\nhello world!\r\n0\r\nX-Trailer: value\r\n\r\n'
  )
  const request3 = Buffer.from(
    'HTTP/1.1 200 OK\r\nDate: Wed, 15 Nov 2023 21:06:00 GMT\r\nConnection: keep-alive\r\nKeep-Alive: timeout=600\r\nContent-Length: 65536\r\n\r\n' +
      Buffer.alloc(64 * 1024, '_').toString()
  )

  testData = [milo, parser, request1, request2, request3]
  return testData
}
