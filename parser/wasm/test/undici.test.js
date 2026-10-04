import assert from 'node:assert/strict'
import { it } from 'node:test'
import { createParser, http, setup } from './helpers.js'

it('undici', t => {
  const message = http(String.raw`
    HTTP/1.1 200 OK\r\n
    Connection: keep-alive\r\n
    Content-Length: 65535\r\n
    Date: Sun, 05 Nov 2023 14:26:18 GMT\r\n
    Keep-Alive: timeout=600\r\n\r\n
    @
  `).replaceAll('@', '-'.repeat(65535))
  const { milo, parser, parse } = createParser(t, setup)

  parse(message)
  assert.notEqual(milo.getState(parser), milo.STATE_ERROR)
  assert.equal(milo.getParsed(parser), BigInt(Buffer.byteLength(message)))
})

it('undici_multiple', t => {
  const message = http(String.raw`
    HTTP/1.1 200 OK\r\n
    Date: Mon, 08 Apr 2024 13:20:53 GMT\r\n
    Connection: keep-alive\r\n
    Keep-Alive: timeout=5\r\n
    Transfer-Encoding: chunked\r\n
    \r\n
    3e80\r\n
    @\r\n
    3e80\r\n
    @\r\n
    0\r\n\r\n
  `).replaceAll('@', '-'.repeat(16000))
  const { milo, parser, parse } = createParser(t, setup)

  parse(message)
  assert.notEqual(milo.getState(parser), milo.STATE_ERROR)
})

it('undici_multibyte', t => {
  const message = http(String.raw`
    HTTP/1.1 200 OK\r\n
    Date: Tue, 09 Apr 2024 10:39:04 GMT\r\n
    Connection: keep-alive\r\n
    Keep-Alive: timeout=5\r\n
    Content-Length: 300010\r\n
    \r\n
    {"asd":"@#"}
  `)
    .replaceAll('@', 'あ'.repeat(50000))
    .replaceAll('#', 'あ'.repeat(50000))
  const { milo, parser, parse } = createParser(t, setup)
  const bytes = Buffer.from(message)

  // Split bytes, not JavaScript characters: a chunk may end inside a UTF-8 sequence.
  for (let offset = 0; offset < bytes.length; offset += 65536) {
    parse(bytes.subarray(offset, offset + 65536))
    assert.notEqual(milo.getState(parser), milo.STATE_ERROR)
  }
})
