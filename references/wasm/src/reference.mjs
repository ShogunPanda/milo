#!/usr/bin/env node

import { isMainThread } from 'node:worker_threads'

export const info = isMainThread ? console.log : () => {}

function extractPayload(context, from, size) {
  return context.input.subarray(from, from + size)
}

function sprintf(format, ...args) {
  return format.replaceAll('{}', () => args.shift())
}

function formatEvent(name) {
  return `"${name}"`
}

function appendOutput(message, context, parser, from, size) {
  const payload =
    typeof from === 'number' && typeof size === 'number' && size > 0
      ? `"${extractPayload(context, from, size).toString('utf-8')}"`
      : 'null'
  info(`{ ${message}, "data": ${payload} }`)
  return 0
}

function event(name, position, context, parser, from, size) {
  return appendOutput(sprintf('"pos": {}, "event": "{}"', position, name), context, parser, from, size)
}

function showSpan(name, context, parser, from, size) {
  if (name === 'method' || name === 'url' || name === 'protocol' || name === 'version') {
    context[name] = extractPayload(context, from, size).toString('utf-8')
  }

  return event(name, context.milo.getPosition(parser), context, parser, from, size)
}

function onStateChange(context, parser, from, size) {
  return appendOutput(
    sprintf(
      '"pos": {}, "event": "state", "state": "{}"',
      context.milo.getPosition(parser),
      context.milo.States[context.milo.getState(parser)]
    ),
    context,
    parser,
    from,
    size
  )
}

function onMessageStart(context, parser, from, size) {
  return appendOutput(
    sprintf(
      '"pos": {}, "event": "begin", "configuration": { "debug": {} }',
      context.milo.getPosition(parser),
      context.milo.FLAG_DEBUG
    ),
    context,
    parser,
    from,
    size
  )
}

function onMessageComplete(context, parser, from, size) {
  return event('complete', context.milo.getPosition(parser), context, parser, from, size)
}

function onError(context, parser, from, size) {
  const errorDescription = context.milo.getErrorDescription(parser)
  let callbackError = context.milo.getCallbackError(parser)

  if (callbackError) {
    callbackError = JSON.stringify({
      type: callbackError.name,
      message: callbackError.message,
      stack: callbackError.stack
    })
  }

  const errorCode = parser.errorCode
  return appendOutput(
    sprintf(
      '"pos": {}, "event": {}, "error_code": {}, "error_code_string": "{}", reason: "{}", callbackError: {}',
      context.milo.getPosition(parser),
      'error',
      errorCode,
      context.milo.Errors[errorCode],
      errorDescription.toString(),
      callbackError
    ),
    context,
    parser,
    from,
    size
  )
}

function onFinish(context, parser, from, size) {
  return event('finish', context.milo.getPosition(parser), context, parser, from, size)
}

function onRequest(context, parser, from, size) {
  return event('request', context.milo.getPosition(parser), context, parser, from, size)
}

function onResponse(context, parser, from, size) {
  return event('response', context.milo.getPosition(parser), context, parser, from, size)
}

function onMethod(context, parser, from, size) {
  return showSpan('method', context, parser, from, size)
}

function onUrl(context, parser, from, size) {
  return showSpan('url', context, parser, from, size)
}

function onProtocol(context, parser, from, size) {
  return showSpan('protocol', context, parser, from, size)
}

function onVersion(context, parser, from, size) {
  return showSpan('version', context, parser, from, size)
}

function onStatus(context, parser, from, size) {
  return showSpan('status', context, parser, from, size)
}

function onReason(context, parser, from, size) {
  return showSpan('reason', context, parser, from, size)
}

function onHeaderName(context, parser, from, size) {
  return showSpan('header_name', context, parser, from, size)
}

function onHeaderValue(context, parser, from, size) {
  return showSpan('header_value', context, parser, from, size)
}

function onHeaders(context, parser, from, size) {
  const position = context.milo.getPosition(parser)
  const chunked = context.milo.hasChunkedTransferEncoding(parser)
  const contentLength = context.milo.getContentLength(parser)
  const method = context.method
  const url = context.url
  const protocol = context.protocol
  const version = context.version

  if (context.milo.getMessageType(parser) === context.milo.MESSAGE_TYPE_RESPONSE) {
    const status = context.milo.getStatus(parser)
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
        parser,
        from,
        size
      )
    } else if (contentLength > 0) {
      return appendOutput(
        sprintf(
          '{}"status": {}, "protocol": "{}", "version": "{}", "body": {}"',
          heading,
          status,
          protocol,
          version,
          contentLength
        ),
        context,
        parser,
        from,
        size
      )
    } else {
      return appendOutput(
        sprintf('{}"status": {}, "protocol": "{}", "version": "{}", "body": null', heading, status, protocol, version),
        context,
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
        context,
        parser,
        from,
        size
      )
    } else if (contentLength > 0) {
      return appendOutput(
        sprintf(
          '{}"method": "{}", "url": "{}", "protocol": "{}", "version": "{}", "body": {}',
          heading,
          method,
          url,
          protocol,
          version,
          contentLength
        ),
        context,
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
        context,
        parser,
        from,
        size
      )
    }
  }
}

function onUpgrade(context, parser, from, size) {
  return event('upgrade', context.milo.getPosition(parser), context, parser, from, size)
}

function onChunkLength(context, parser, from, size) {
  return showSpan('chunk_length', context, parser, from, size)
}

function onChunkExtensionName(context, parser, from, size) {
  return showSpan('chunk_extensions_name', context, parser, from, size)
}

function onChunkExtensionValue(context, parser, from, size) {
  return showSpan('chunk_extension_value', context, parser, from, size)
}

function onChunk(context, parser, from, size) {
  return event('chunk', context.milo.getPosition(parser), context, parser, from, size)
}

function onBody(context, parser, from, size) {
  return event('body', context.milo.getPosition(parser), context, parser, from, size)
}

function onData(context, parser, from, size) {
  return showSpan('data', context, parser, from, size)
}

function onTrailerName(context, parser, from, size) {
  return showSpan('trailer_name', context, parser, from, size)
}

function onTrailerValue(context, parser, from, size) {
  return showSpan('trailer_value', context, parser, from, size)
}

function onTrailers(context, parser, from, size) {
  return event('trailers', context.milo.getPosition(parser), context, parser, from, size)
}

async function main() {
  const configuration = process.env.CONFIGURATION ?? process.argv[2]
  const { milo } = await import(`../../../parser/dist/wasm/${configuration}/index.js`)
  const parser = milo.create()
  const context = { milo }

  milo.setOnStateChange(parser, onStateChange.bind(null, context))
  milo.setOnError(parser, onError.bind(null, context))
  milo.setOnFinish(parser, onFinish.bind(null, context))
  milo.setOnRequest(parser, onRequest.bind(null, context))
  milo.setOnResponse(parser, onResponse.bind(null, context))
  milo.setOnMessageStart(parser, onMessageStart.bind(null, context))
  milo.setOnMessageComplete(parser, onMessageComplete.bind(null, context))
  milo.setOnMethod(parser, onMethod.bind(null, context))
  milo.setOnUrl(parser, onUrl.bind(null, context))
  milo.setOnProtocol(parser, onProtocol.bind(null, context))
  milo.setOnVersion(parser, onVersion.bind(null, context))
  milo.setOnStatus(parser, onStatus.bind(null, context))
  milo.setOnReason(parser, onReason.bind(null, context))
  milo.setOnHeaderName(parser, onHeaderName.bind(null, context))
  milo.setOnHeaderValue(parser, onHeaderValue.bind(null, context))
  milo.setOnHeaders(parser, onHeaders.bind(null, context))
  milo.setOnUpgrade(parser, onUpgrade.bind(null, context))
  milo.setOnChunkLength(parser, onChunkLength.bind(null, context))
  milo.setOnChunkExtensionName(parser, onChunkExtensionName.bind(null, context))
  milo.setOnChunkExtensionValue(parser, onChunkExtensionValue.bind(null, context))
  milo.setOnChunk(parser, onChunk.bind(null, context))
  milo.setOnBody(parser, onBody.bind(null, context))
  milo.setOnData(parser, onData.bind(null, context))
  milo.setOnTrailerName(parser, onTrailerName.bind(null, context))
  milo.setOnTrailerValue(parser, onTrailerValue.bind(null, context))
  milo.setOnTrailers(parser, onTrailers.bind(null, context))

  const request1 = 'GET / HTTP/1.1\r\n\r\n'
  const request2 =
    'HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: x-trailer\r\n\r\nc;need=love\r\nhello world!\r\n0\r\nX-Trailer: value\r\n\r\n'

  const ptr = milo.alloc(1000)
  const buffer = Buffer.from(milo.memory.buffer, ptr, 1000)
  context.input = buffer
  buffer.set(Buffer.from(request1), 0)

  let consumed = milo.parse(parser, ptr, request1.length)
  info(
    `{ "pos": ${milo.getPosition(parser)}, "consumed": ${consumed}, "state": "${milo.States[milo.getState(parser)]}" }`
  )

  info('\n------------------------------------------------------------------------------------------\n')

  buffer.set(Buffer.from(request2), 0)
  consumed = milo.parse(parser, ptr, request2.length)
  info(
    `{ "pos": ${milo.getPosition(parser)}, "consumed": ${consumed}, "state": "${milo.States[milo.getState(parser)]}" }`
  )

  milo.destroy(parser)
}

await main()
