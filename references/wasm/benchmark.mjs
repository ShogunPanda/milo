import cronometro from 'cronometro'

function extractPayload(parser, from, size) {
  return Buffer.from(parser.context.input.slice(from, from + size))
}

function sprintf(format, ...args) {
  return format.replaceAll('{}', () => args.shift())
}

function formatEvent(name) {
  return `"${name}"`
}

function appendOutput(message, parser, from, size) {
  const payload = typeof from === 'number' ? `"${extractPayload(parser, from, size).toString('utf-8')}"` : 'null'
  // console.log(`{ ${message}, "data": ${payload} }`)
  return 0
}

function event(parser, name, from, size) {
  return appendOutput(sprintf('"pos": {}, "event": "{}"', parser.position, name), parser, from, size)
}

function showSpan(parser, name, from, size) {
  if (name == 'method' || name == 'url' || name == 'protocol' || name == 'version') {
    parser.context[name] = extractPayload(parser, from, size).toString('utf-8')
  }

  return event(parser, name, from, size)
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
  const flags = this.flags()

  return appendOutput(
    sprintf(
      '"pos": {}, "event": "begin", "configuration": { "debug": {}, "all-callbacks": {} }',
      parser.position,
      flags.debug,
      flags.all_callbacks
    ),
    parser,
    from,
    size
  )
}

function onMessageComplete(parser, from, size) {
  return event(parser, 'complete', from, size)
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
  return event(parser, 'finish', from, size)
}

function onRequest(parser, from, size) {
  return event(parser, 'request', from, size)
}

function onResponse(parser, from, size) {
  return event(parser, 'response', from, size)
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
  const method = parser.context.method
  const url = parser.context.url
  const protocol = parser.context.protocol
  const version = parser.context.version

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
  return event(parser, 'upgrade', from, size)
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

function onBody(parser, from, size) {
  return event(parser, 'body', from, size)
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
  return event(parser, 'trailers', from, size)
}

async function main() {
  const milo = await import(`./lib/release-default/milo.js`)

  const parser = new milo.Parser()
  parser.context = {}

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
  parser.setOnBody(onBody.bind(milo, parser))
  parser.setOnData(onData.bind(milo, parser))
  parser.setOnTrailerName(onTrailerName.bind(milo, parser))
  parser.setOnTrailerValue(onTrailerValue.bind(milo, parser))
  parser.setOnTrailers(onTrailers.bind(milo, parser))

  const request1 = Buffer.from('GET / HTTP/1.1\r\n\r\n')
  const request2 = Buffer.from(
    'HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: x-trailer\r\n\r\nc;need=love\r\nhello world!\r\n0\r\nX-Trailer: value\r\n\r\n'
  )

  const buffer = milo.__wasm.__wbindgen_export_0(4096, 1) >>> 0

  new Uint8Array(milo.__wasm.memory.buffer, buffer, 4096).set(request1)
  parser.context.input = Buffer.from(request1)
  let consumed = parser.parse(buffer, request1.length)

  new Uint8Array(milo.__wasm.memory.buffer, buffer, 4096).set(request2)
  parser.context.input = Buffer.from(request2)
  consumed = parser.parse(buffer, request2.length)
}

await main()
await cronometro({ main })
