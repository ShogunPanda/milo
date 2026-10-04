import assert from 'node:assert/strict'
import { it } from 'node:test'
import { createParser, http, setup } from './helpers.js'

it('upgrade_connect_request', t => {
  const { milo, parser, parse } = createParser(t, setup)
  const message1 = http(String.raw`
    CONNECT example.com HTTP/1.1\r\n
    Host: example.com\r\n
    Content-Length: 3\r\n
    \r\n
    abc\r\n\r\n
  `)
  const message2 = 'abc\r\n\r\n'

  assert.equal(parse(message1), 70)
  assert.equal(milo.getState(parser), milo.STATE_TUNNEL)
  assert.equal(parse(message2), 0)
  assert.equal(milo.getState(parser), milo.STATE_TUNNEL)
})

it('upgrade_connection_upgrade', t => {
  const { milo, parser, parse } = createParser(t, setup)
  const message1 = http(String.raw`
    POST / HTTP/1.1\r\n
    Host: example.com\r\n
    Connection: upgrade\r\n
    Upgrade: websocket\r\n
    Content-Length: 3\r\n
    \r\n
    abc\r\n\r\n
  `)
  const message2 = 'abc\r\n\r\n'

  assert.equal(parse(message1), message1.length - 4)
  assert.equal(milo.getState(parser), milo.STATE_TUNNEL)
  assert.equal(parse(message2), 0)
  assert.equal(milo.getState(parser), milo.STATE_TUNNEL)
})

it('upgrade_http_101', t => {
  const { milo, parser, parse } = createParser(t, setup)
  const message1 = http(String.raw`
    HTTP/1.1 101 Switching Protocols\r\n
    hello: world\r\n
    connection: upgrade\r\n
    upgrade: websocket\r\n
    \r\n
    Body
  `)
  const message2 = 'abc\r\n\r\n'

  assert.equal(parse(message1), message1.length - 4)
  assert.equal(milo.getState(parser), milo.STATE_TUNNEL)
  assert.equal(parse(message2), 0)
  assert.equal(milo.getState(parser), milo.STATE_TUNNEL)
})
