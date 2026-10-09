import assert from 'node:assert/strict'
import { it } from 'node:test'
import { createParser, http, setup } from './helpers.js'

// Keep case names aligned with parser/tests/basic.rs for coverage comparisons.
it('basic_error_description_is_clamped_and_terminated', t => {
  const { milo, parser } = createParser(t, setup)
  milo.fail(parser, milo.ERROR_UNEXPECTED_CHARACTER, 'a'.repeat(300))

  let memory = new Uint8Array(milo.memory.buffer)
  assert.equal(memory[parser + milo.PARSER_FIELD_ERROR_DESCRIPTION_LEN], 254)
  assert.equal(memory[parser + milo.PARSER_FIELD_ERROR_DESCRIPTION + 254], 0)
  assert.equal(milo.getErrorDescription(parser), 'a'.repeat(254))

  milo.reset(parser, false)
  memory = new Uint8Array(milo.memory.buffer)
  assert.equal(memory[parser + milo.PARSER_FIELD_ERROR_DESCRIPTION_LEN], 0)
  assert.equal(memory[parser + milo.PARSER_FIELD_ERROR_DESCRIPTION], 0)
})

it('basic_disable_autodetect', t => {
  const { milo, parser, parse } = createParser(t, setup)
  const request = http(String.raw`
        PUT /url HTTP/1.1\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n\r\n
      `)
  const response = http(String.raw`
        HTTP/1.1 200 OK\r\n
        Header1: Value1\r\n
        Header2: Value2\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n\r\n
      `)

  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, true)
  parse(response)
  assert.equal(milo.getState(parser), milo.STATE_ERROR)
  milo.reset(parser, false)

  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, false)
  parse(request)
  assert.equal(milo.getState(parser), milo.STATE_ERROR)
})

it('basic_incomplete_string_1', t => {
  const { milo, parser, parse } = createParser(t, setup)
  for (const sample of ['GET / HTTP/1.1\r', '1.1\r\n', 'Head', 'Header:', 'Value', 'Value\r\n\r\n']) {
    assert.equal(parse(sample), 0)
  }
  assert.equal(milo.getState(parser), milo.STATE_ERROR)
})

it('basic_incomplete_string_2', t => {
  const { milo, parser, parse } = createParser(t, setup)
  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, true)
  const message = 'GET / HTTP/1.1\r\nHost: foo\r\n\r\n'

  assert.equal(parse('GE'), 0)
  assert.equal(parse(message), message.length)
  assert.notEqual(milo.getState(parser), milo.STATE_ERROR)
})

it('basic_incomplete_string_automanaged_1', t => {
  const { milo, parser, parse } = createParser(t, setup)
  milo.setShouldManageUnconsumed(parser, true)
  const message = 'GET / HTTP/1.1\r\nHeader: Value\r\n\r\n'
  const samples = [
    message.slice(0, 15),
    message.slice(15, 16),
    message.slice(16, 20),
    message.slice(20, 24),
    message.slice(24, 29),
    message.slice(29)
  ]
  const consumed = [0, 16, 0, 0, 0, message.length - 16]

  for (let pass = 1; pass <= 2; pass++) {
    for (let i = 0; i < samples.length; i++) {
      assert.equal(parse(samples[i]), consumed[i])
    }
    assert.notEqual(milo.getState(parser), milo.STATE_ERROR)
    assert.equal(milo.getParsed(parser), BigInt(message.length * pass))
    if (pass === 1) {
      // Reset must preserve both the parsed counter and automatic buffering.
      milo.reset(parser, true)
    }
  }
})

it('basic_incomplete_string_automanaged_2', t => {
  const { milo, parser, parse } = createParser(t, setup)
  milo.setShouldManageUnconsumed(parser, true)
  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, true)
  const message = 'GET / HTTP/1.1\r\nHost: foo\r\n\r\n'

  parse(message.slice(0, 2))
  parse(message.slice(2))
  assert.notEqual(milo.getState(parser), milo.STATE_ERROR)
  assert.equal(milo.getParsed(parser), BigInt(message.length))
})

it('basic_sample_multiple_requests', t => {
  const { milo, parser, parse } = createParser(t, setup)
  parse(
    http(String.raw`
        POST /chunked_w_unicorns_after_length HTTP/1.1\r\n
        Transfer-Encoding: chunked\r\n
        \r\n
        5;ilovew3;somuchlove=aretheseparametersfor\r\n
        hello\r\n
        7;blahblah;blah\r\n
        \s world\r\n
        0\r\n\r\n
        \r\n
        POST / HTTP/1.1\r\n
        Host: www.example.com\r\n
        Content-Type: application/x-www-form-urlencoded\r\n
        Content-Length: 4\r\n
        \r\n
        q=42\r\n
        \r\n
        GET / HTTP/1.1\r\n\r\n
      `)
  )
  assert.notEqual(milo.getState(parser), milo.STATE_ERROR)
})

it('basic_connection_close', t => {
  const { milo, parser, parse } = createParser(t, setup)
  parse(
    http(String.raw`
        POST /chunked_w_unicorns_after_length HTTP/1.1\r\n
        Connection: close\r\n
        Transfer-Encoding: chunked\r\n
        \r\n
        5;ilovew3;somuchlove=aretheseparametersfor\r\n
        hello\r\n
        7;blahblah;blah\r\n
        \s world\r\n
        0\r\n\r\n
      `)
  )
  assert.equal(milo.getState(parser), milo.STATE_FINISH)
})

it('basic_max_body_payload_content_length', t => {
  const { milo, parser, parse } = createParser(t, setup)
  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, false)
  milo.setMaxBodyPayload(parser, 3n)
  const message = 'HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\nabcdef'
  const bodyStart = message.indexOf('\r\n\r\n') + 4

  assert.equal(parse(message), bodyStart + 3)
  assert.equal(milo.getRemainingContentLength(parser), 3n)
  assert.equal(milo.isPaused(parser), false)
  assert.equal(parse(message.slice(bodyStart + 3)), 3)
  assert.equal(milo.getRemainingContentLength(parser), 0n)
})

it('basic_max_body_payload_chunked', t => {
  const { milo, parser, parse } = createParser(t, setup)
  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, false)
  milo.setMaxBodyPayload(parser, 3n)
  const message = 'HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n6\r\nabcdef\r\n0\r\n\r\n'
  const chunkDataStart = message.indexOf('\r\n\r\n6\r\n') + 7

  assert.equal(parse(message), chunkDataStart + 3)
  assert.equal(milo.getRemainingChunkSize(parser), 3n)
  assert.equal(milo.isPaused(parser), false)
  const remaining = message.slice(chunkDataStart + 3)
  assert.equal(parse(remaining), remaining.length)
  assert.equal(milo.getRemainingChunkSize(parser), 0n)
})

it('basic_max_body_payload_no_length', t => {
  const { milo, parser, parse } = createParser(t, setup)
  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, false)
  milo.setMaxBodyPayload(parser, 3n)
  const message = 'HTTP/1.1 200 OK\r\n\r\nabcdef'
  const bodyStart = message.indexOf('\r\n\r\n') + 4

  assert.equal(parse(message), bodyStart + 3)
  assert.equal(milo.isPaused(parser), false)
  assert.equal(parse(message.slice(bodyStart + 3)), 3)
})

it('basic_max_body_payload_zero_is_unlimited', t => {
  const { milo, parser, parse } = createParser(t, setup)
  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, false)
  milo.setMaxBodyPayload(parser, 0n)
  const message = 'HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\nabcdef'

  assert.equal(parse(message), message.length)
  assert.equal(milo.getRemainingContentLength(parser), 0n)
})

it('basic_suspend_after_headers_content_length', t => {
  const { milo, parser, parse } = createParser(t, setup)
  milo.setShouldSuspendAfterHeaders(parser, true)
  const message = 'POST / HTTP/1.1\r\nContent-Length: 6\r\n\r\nabcdef'
  const bodyStart = message.indexOf('\r\n\r\n') + 4

  assert.equal(parse(message), bodyStart)
  assert.equal(milo.getState(parser), milo.STATE_BODY_DECISION)
  assert.equal(milo.isPaused(parser), false)
  assert.equal(parse(message.slice(bodyStart)), message.length - bodyStart)
  assert.equal(milo.getState(parser), milo.STATE_START)
})

it('basic_suspend_after_headers_emits_headers_once', t => {
  const { milo, parser, parse, headers } = createParser(t, setup)
  milo.setActiveCallbacks(parser, milo.CALLBACK_ACTIVE_ON_HEADERS)
  milo.setShouldSuspendAfterHeaders(parser, true)
  const message = 'POST / HTTP/1.1\r\nContent-Length: 6\r\n\r\nabcdef'
  const bodyStart = message.indexOf('\r\n\r\n') + 4

  assert.equal(parse(message), bodyStart)
  assert.equal(parse(message.slice(bodyStart)), message.length - bodyStart)
  assert.equal(headers.length, 1)
})

it('basic_suspend_after_headers_zero_body_completes_on_empty_parse', t => {
  const { milo, parser, parse } = createParser(t, setup)
  milo.setShouldSuspendAfterHeaders(parser, true)
  const message = 'HTTP/1.1 204 No Content\r\n\r\n'

  assert.equal(parse(message), message.length)
  assert.equal(milo.getState(parser), milo.STATE_BODY_DECISION)
  assert.equal(parse(''), 0)
  assert.equal(milo.getState(parser), milo.STATE_START)
})

it('basic_complete_after_suspend_after_headers', t => {
  const { milo, parser, parse } = createParser(t, setup)
  milo.setShouldSuspendAfterHeaders(parser, true)
  const message = 'POST / HTTP/1.1\r\nContent-Length: 6\r\n\r\nabcdef'
  const bodyStart = message.indexOf('\r\n\r\n') + 4

  assert.equal(parse(message), bodyStart)
  assert.equal(milo.getState(parser), milo.STATE_BODY_DECISION)
  milo.complete(parser)
  assert.equal(milo.getState(parser), milo.STATE_START)
  assert.equal(milo.getErrorCode(parser), milo.ERROR_NONE)
})

it('basic_managed_input_does_not_retain_tunnel_data', t => {
  const { milo, parser, parse } = createParser(t, setup)
  milo.setShouldManageUnconsumed(parser, true)
  const headers = 'CONNECT example.com:443 HTTP/1.1\r\n\r\n'

  assert.equal(parse(`${headers}opaque tunnel data`), headers.length)
  assert.equal(milo.getState(parser), milo.STATE_TUNNEL)
  let view = new DataView(milo.memory.buffer)
  assert.equal(view.getUint32(parser + milo.PARSER_FIELD_UNCONSUMED_LEN, true), 0)

  assert.equal(parse('more tunnel data'), 0)
  view = new DataView(milo.memory.buffer)
  assert.equal(view.getUint32(parser + milo.PARSER_FIELD_UNCONSUMED_LEN, true), 0)

  const suspended = createParser(t, setup)
  suspended.milo.setShouldManageUnconsumed(suspended.parser, true)
  suspended.milo.setShouldSuspendAfterHeaders(suspended.parser, true)
  const upgradeHeaders = 'GET / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: websocket\r\n\r\n'
  assert.equal(suspended.parse(`${upgradeHeaders}opaque tunnel data`), upgradeHeaders.length)
  view = new DataView(suspended.milo.memory.buffer)
  assert.ok(view.getUint32(suspended.parser + suspended.milo.PARSER_FIELD_UNCONSUMED_LEN, true) > 0)

  suspended.milo.complete(suspended.parser)
  assert.equal(suspended.milo.getState(suspended.parser), suspended.milo.STATE_TUNNEL)
  view = new DataView(suspended.milo.memory.buffer)
  assert.equal(view.getUint32(suspended.parser + suspended.milo.PARSER_FIELD_UNCONSUMED_LEN, true), 0)
})

it('basic_complete_rejects_invalid_state', t => {
  const { milo, parser } = createParser(t, setup)
  milo.complete(parser)
  assert.equal(milo.getState(parser), milo.STATE_ERROR)
  assert.equal(milo.getErrorCode(parser), milo.ERROR_UNEXPECTED_STATE)
  assert.equal(milo.getErrorDescription(parser), 'Invalid state')
})

it('basic_event_buffer_full_stops_parsing', t => {
  const { milo, parser, parse } = createParser(t, setup)
  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, false)
  milo.setActiveCallbacks(parser, 0n)
  milo.setActiveEvents(parser, milo.EVENT_ACTIVE_ON_HEADER_NAME | milo.EVENT_ACTIVE_ON_HEADER_VALUE)
  let message = 'HTTP/1.1 200 OK\r\n'
  for (let i = 0; i < 4000; i++) {
    message += `Header${i}: value\r\n`
  }
  message += '\r\n'

  assert.ok(parse(message) < message.length)
  assert.equal(milo.isPaused(parser), false)
  assert.equal(milo.getErrorCode(parser), milo.ERROR_NONE)
})

it('basic_event_buffer_full_resumes_completion', () => {
  for (const close of [false, true]) {
    const received = []
    const milo = setup({
      on_header_value () {},
      on_headers () {},
      on_data (parser, at, len) {
        received.push(['data', len])
      },
      on_body () {
        received.push(['body'])
      },
      on_message_complete () {
        received.push(['complete'])
      },
      on_reset () {
        received.push(['reset'])
      },
      on_finish () {
        received.push(['finish'])
      }
    })
    const parser = milo.create()
    const message = Buffer.from(
      'HTTP/1.1 200 OK\r\n' +
        'X: a\r\n'.repeat(7276) +
        (close ? 'Connection: close\r\n' : '') +
        'Content-Length: 2\r\n\r\nok'
    )
    const ptr = milo.alloc(message.length)
    try {
      milo.setActiveCallbacks(
        parser,
        milo.CALLBACK_ACTIVE_ON_HEADER_VALUE |
          milo.CALLBACK_ACTIVE_ON_HEADERS |
          milo.CALLBACK_ACTIVE_ON_DATA |
          milo.CALLBACK_ACTIVE_ON_BODY |
          milo.CALLBACK_ACTIVE_ON_MESSAGE_COMPLETE |
          milo.CALLBACK_ACTIVE_ON_RESET |
          milo.CALLBACK_ACTIVE_ON_FINISH
      )
      new Uint8Array(milo.memory.buffer, ptr, message.length).set(message)
      const consumed = milo.parse(parser, ptr, message.length)
      assert.equal(consumed, message.length - 2)
      assert.equal(milo.getRemainingContentLength(parser), 2n)
      assert.equal(milo.getState(parser), milo.STATE_BODY_VIA_CONTENT_LENGTH)
      assert.deepEqual(received, [])
      assert.equal(milo.parse(parser, ptr + consumed, message.length - consumed), 2)
      assert.deepEqual(received, [['data', 2], ['body'], ['complete'], ['reset'], ...(close ? [['finish']] : [])])
      assert.equal(milo.getErrorCode(parser), milo.ERROR_NONE)
      assert.equal(milo.getState(parser), close ? milo.STATE_FINISH : milo.STATE_START)
    } finally {
      milo.destroy(parser)
      milo.dealloc(ptr, message.length)
    }
  }
})

it('basic_sample_multiple_responses', t => {
  const { milo, parser, parse } = createParser(t, setup)
  parse(
    http(String.raw`
        HTTP/1.1 200 OK\r\n
        Header1: Value1\r\n
        Header2: Value2\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n\r\n
        HTTP/1.1 200 OK\r\n
        Header1: Value1\r\n
        Header2: Value2\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n
        HTTP/1.1 200 OK\r\n
        Header1: Value1\r\n
        Header2: Value2\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n\r\n
      `)
  )
  assert.notEqual(milo.getState(parser), milo.STATE_ERROR)
})

it('basic_trailers', t => {
  const { milo, parser, parse } = createParser(t, setup)
  parse(
    http(String.raw`
        POST /chunked_w_unicorns_after_length HTTP/1.1\r\n
        Transfer-Encoding: chunked\r\n
        Trailer: host,cache-control\r\n
        \r\n
        5;ilovew3;somuchlove="arethesepara\"metersfor";another="1111\"2222\"3333"\r\n
        hello\r\n
        7;blahblah;blah;somuchlove="arethesepara"\r\n
        \s world\r\n
        0\r\n
        Host: example.com\r\n
        Cache-Control: private\r\n\r\n
      `)
  )
  assert.notEqual(milo.getState(parser), milo.STATE_ERROR)
})

it('basic_incomplete_body', t => {
  const { milo, parser, parse } = createParser(t, setup)
  for (const sample of ['POST / HTTP/1.1\r\nContent-Length:10\r\n\r\n12345', '67', '890\r\n']) {
    assert.equal(parse(sample), sample.length)
  }
  assert.notEqual(milo.getState(parser), milo.STATE_ERROR)
})

it('basic_incomplete_chunk', t => {
  const { milo, parser, parse } = createParser(t, setup)
  const samples = [
    'POST / HTTP/1.1\r\nTransfer-Encoding:chunked\r\nTrailer: x-foo\r\n\r\na\r\n12345',
    '67',
    '890\r\n0\r\nx-foo:value\r\n\r\n'
  ]
  for (const sample of samples) {
    assert.equal(parse(sample), sample.length)
  }
  assert.notEqual(milo.getState(parser), milo.STATE_ERROR)
})

it('basic_connection_header', t => {
  const { milo, parser, parse } = createParser(t, setup)
  parse('PUT /url HTTP/1.1\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc')
  assert.equal(milo.getState(parser), milo.STATE_FINISH)
  milo.reset(parser, false)
  parse('PUT /url HTTP/1.1\r\nContent-Length: 3\r\n\r\nabc')
  assert.equal(milo.getState(parser), milo.STATE_START)
})

it('basic_pause_and_resume', t => {
  const { milo, parser, parse } = createParser(t, setup)
  const sample1 = 'PUT /url HTTP/1.1\r\nContent-Length: 3\r\n'
  const sample2 = '\r\nabc'

  assert.equal(milo.isPaused(parser), false)
  assert.equal(parse(sample1), sample1.length)
  assert.equal(milo.isPaused(parser), false)
  milo.pause(parser)
  assert.equal(milo.isPaused(parser), true)
  assert.equal(parse(sample2), 0)
  assert.equal(milo.isPaused(parser), true)
  milo.resume(parser)
  assert.equal(milo.isPaused(parser), false)
  assert.equal(parse(sample2), sample2.length)
  assert.equal(milo.isPaused(parser), false)
  assert.notEqual(milo.getState(parser), milo.STATE_ERROR)
})

it('basic_restart', t => {
  const { milo, parser, parse } = createParser(t, setup)
  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, false)
  const response = http(String.raw`
        HTTP/1.1 200 OK\r\n
        Header1: Value1\r\n
        Header2: Value2\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n\r\n
        HTTP/1.1 200 OK\r\n
        Header1: Value1\r\n
        Header2: Value2\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n
        HTTP/1.1 200 OK\r\n
        Header1: Value1\r\n
        Header2: Value2\r\n
        Content-Length: 3\r\n
        \r\n
        abc
      `)
  const request = 'PUT /url HTTP/1.1\r\nContent-Length: 3\r\nConnection: keep-alive\r\n\r\nabc'

  parse(response)
  assert.equal(milo.getState(parser), milo.STATE_START)
  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, true)
  milo.reset(parser, false)
  parse(request)
  assert.equal(milo.getState(parser), milo.STATE_START)
})

it('basic_finish_logic', t => {
  const { milo, parser, parse } = createParser(t, setup)
  assert.equal(milo.getState(parser), milo.STATE_START)
  milo.finish(parser)
  assert.equal(milo.getState(parser), milo.STATE_FINISH)
  milo.reset(parser, false)

  parse('PUT /url HTTP/1.1\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc')
  assert.equal(milo.getState(parser), milo.STATE_FINISH)
  milo.finish(parser)
  assert.equal(milo.getState(parser), milo.STATE_FINISH)
  milo.reset(parser, false)

  parse('PUT /url HTTP/1.1\r\nContent-Length: 3\r\n\r\nabc')
  assert.equal(milo.getState(parser), milo.STATE_START)
  milo.finish(parser)
  assert.equal(milo.getState(parser), milo.STATE_FINISH)
  milo.reset(parser, false)

  parse('PUT /url HTTP/1.1\r\n')
  assert.equal(milo.getState(parser), milo.STATE_HEADER)
  milo.finish(parser)
  assert.equal(milo.getState(parser), milo.STATE_ERROR)
})

it('basic_empty_fields', t => {
  const { milo, parser, parse } = createParser(t, setup)
  parse(
    http(String.raw`
        POST / HTTP/1.1\r\n
        Transfer-Encoding: chunked\r\n
        Content-Type: \r\n
        Trailer: host\r\n
        \r\n
        0\r\n
        Host:\r\n\r\n
      `)
  )
  assert.notEqual(milo.getState(parser), milo.STATE_ERROR)
})

it('basic_space_after_header_name', t => {
  const { milo, parser, parse } = createParser(t, setup)
  parse('PUT /url HTTP/1.1\r\nContent-Length : 3\r\n\r\nabc\r\n\r\n')
  assert.equal(milo.getState(parser), milo.STATE_ERROR)
})

it('basic_response_204_has_no_body', t => {
  const { milo, parser, parse } = createParser(t, setup)
  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, false)
  parse('HTTP/1.1 204 No content\r\n\r\nHTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok')
  assert.equal(milo.getState(parser), milo.STATE_START)
})
