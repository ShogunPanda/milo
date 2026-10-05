import assert from 'node:assert/strict'
import { it } from 'node:test'
import { createParser, setup } from './helpers.js'

it('issue-26 - preserve_events_before_error', () => {
  const suffix = 'HTTP/9.9 garbage\r\n\r\n'
  for (const chunked of [false, true]) {
    for (const split of [false, true]) {
      for (const errors of [false, true]) {
        for (const callbacks of [false, true]) {
          // The largest case must suspend before completion and resume in a fresh batch.
          for (const padding of [0, 7276, 7277]) {
            const label = JSON.stringify({ chunked, split, errors, callbacks, padding })
            const received = []
            const milo = setup({
              on_headers () {
                received.push('headers')
              },
              on_message_complete () {
                received.push('complete')
              },
              on_error () {
                received.push('error')
              },
              on_header_value () {
                received.push('value')
              }
            })
            const parser = milo.create()
            const response =
              'HTTP/1.1 200 OK\r\n' +
              'X: a\r\n'.repeat(padding) +
              (chunked ? 'Transfer-Encoding: chunked\r\n\r\n2\r\nok\r\n0\r\n\r\n' : 'Content-Length: 2\r\n\r\nok')
            try {
              milo.setShouldAutodetect(parser, false)
              milo.setIsRequest(parser, false)
              const mask =
                milo.EVENT_ACTIVE_ON_HEADERS |
                milo.EVENT_ACTIVE_ON_MESSAGE_COMPLETE |
                milo.EVENT_ACTIVE_ON_HEADER_VALUE |
                (errors ? milo.EVENT_ACTIVE_ON_ERROR : 0n)
              milo.setActiveEvents(parser, callbacks ? 0n : mask)
              milo.setActiveCallbacks(parser, callbacks ? mask : 0n)
              for (const input of split ? [response, suffix] : [response + suffix]) {
                const bytes = Buffer.from(input)
                const ptr = milo.alloc(bytes.length)
                try {
                  new Uint8Array(milo.memory.buffer, ptr, bytes.length).set(bytes)
                  let offset = 0
                  while (offset < bytes.length && milo.getState(parser) !== milo.STATE_ERROR) {
                    const consumed = milo.parse(parser, ptr + offset, bytes.length - offset)
                    if (!callbacks) {
                      const view = new DataView(milo.memory.buffer)
                      let cursor = view.getUint32(parser + milo.ParserFields.EVENTS, true)
                      const end = cursor + 65536
                      while (view.getUint8(cursor) !== milo.EVENT_END) {
                        const type = view.getUint8(cursor)
                        if (type === milo.EVENT_HEADERS) {
                          received.push('headers')
                          cursor += 19
                        } else if (type === milo.EVENT_ERROR) {
                          received.push('error')
                          assert.equal(view.getUint8(cursor + 5), milo.ERROR_UNSUPPORTED_HTTP_VERSION, label)
                          cursor += 6
                        } else {
                          assert.ok(type === milo.EVENT_MESSAGE_COMPLETE || type === milo.EVENT_HEADER_VALUE, label)
                          received.push(type === milo.EVENT_MESSAGE_COMPLETE ? 'complete' : 'value')
                          cursor += 9
                        }
                        assert.ok(cursor < end, label)
                      }
                    }
                    assert.ok(consumed > 0 || milo.getState(parser) === milo.STATE_ERROR, label)
                    offset += consumed
                  }
                } finally {
                  milo.dealloc(ptr, bytes.length)
                }
              }
              assert.deepEqual(
                received,
                [...Array(padding + 1).fill('value'), 'headers', 'complete', ...(errors ? ['error'] : [])],
                label
              )
              assert.equal(milo.getState(parser), milo.STATE_ERROR, label)
              assert.equal(milo.getErrorCode(parser), milo.ERROR_UNSUPPORTED_HTTP_VERSION, label)
              const count = received.length
              assert.equal(milo.parse(parser, parser, 0), 0, label)
              assert.equal(received.length, count, label)
              assert.equal(milo.getState(parser), milo.STATE_ERROR, label)
            } finally {
              milo.destroy(parser)
            }
          }
        }
      }
    }
  }
})

function responseParser (t) {
  const context = createParser(t, setup)
  context.milo.setShouldAutodetect(context.parser, false)
  context.milo.setIsRequest(context.parser, false)
  return context
}

function fieldMessages (value) {
  return [
    ['HTTP/1.1 200 OK\r\nX-Long: ', '\r\nContent-Length: 0\r\n\r\n'],
    ['HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX-Long: ', '\r\n\r\n'],
    ['HTTP/1.1 200 ', '\r\nContent-Length: 0\r\n\r\n']
  ].map(([prefix, suffix]) => Buffer.concat([Buffer.from(prefix), value, Buffer.from(suffix)]))
}

// Cover SIMD block boundaries and scalar tails in both field scanners.
it('issue-22 - field_values_reject_controls', t => {
  const { milo, parser, parse } = responseParser(t)
  milo.setActiveCallbacks(parser, 0n)
  const controls = [...Array.from({ length: 32 }, (_, byte) => byte), 0x7f].filter(byte => byte !== 9)

  for (const byte of controls) {
    for (const offset of [0, 7, 8, 15, 16, 20, 31, 32]) {
      for (const trailing of [0, 20]) {
        const value = Buffer.concat([Buffer.alloc(offset, 'a'), Buffer.from([byte]), Buffer.alloc(trailing, 'a')])
        for (const message of fieldMessages(value)) {
          milo.reset(parser, false)
          parse(message)
          assert.equal(milo.getState(parser), milo.STATE_ERROR, `Accepted invalid field: ${message.toString('hex')}`)
        }
      }
    }
  }
})

// HTAB and every obs-text byte remain valid, including across SIMD boundaries.
it('issue-22 - field_values_allow_tab_and_obs_text', t => {
  const { milo, parser, parse } = responseParser(t)
  // Keep the callback configuration identical to the raw-byte Rust regression.
  milo.setActiveCallbacks(parser, 0n)
  const allowed = [9, ...Array.from({ length: 128 }, (_, byte) => byte + 0x80)]

  for (const byte of allowed) {
    for (const offset of [0, 7, 8, 15, 16, 20, 31, 32]) {
      for (const trailing of [0, 20]) {
        const value = Buffer.concat([Buffer.alloc(offset, 'a'), Buffer.from([byte]), Buffer.alloc(trailing, 'a')])
        for (const message of fieldMessages(value)) {
          milo.reset(parser, false)
          parse(message)
          assert.notEqual(milo.getState(parser), milo.STATE_ERROR, `Rejected valid field: ${message.toString('hex')}`)
          assert.equal(milo.getErrorCode(parser), milo.ERROR_NONE)
        }
      }
    }
  }
})

// Bare LF is rejected in HTTP framing.
it('issue-22 - bare_lf_rejected', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nHeader: value\nContent-Length: 0\r\n\r\n')
  assert.equal(milo.getState(parser), milo.STATE_ERROR)
})

it('issue-24 - memory_deallocation', async () => {
  const milo = setup()
  const size = 65536

  // Warm up the allocator before measuring the linear memory high-water mark.
  for (let i = 0; i < 100; i++) {
    const ptr = milo.alloc(size)
    milo.dealloc(ptr, size)
  }

  const before = milo.memory.buffer.byteLength
  for (let batch = 0; batch < 10; batch++) {
    for (let i = 0; i < 100; i++) {
      const ptr = milo.alloc(size)
      milo.dealloc(ptr, size)
    }

    // WASM memory cannot shrink; freeing buffers must allow subsequent allocations to reuse it.
    assert.equal(milo.memory.buffer.byteLength, before, `memory grew in batch ${batch + 1}`)
  }
})

it('issue-25 - headers_upgrade_metadata', () => {
  const cases = [
    ...[100, 101, 103, 200, 204, 301, 304, 400, 426, 500].map(status => ({
      start: `HTTP/1.1 ${status} Test`,
      status,
      request: false,
      connect: false
    })),
    { start: 'POST / HTTP/1.1', request: true, connect: false },
    { start: 'CONNECT example.com:443 HTTP/1.1', request: true, connect: true },
    { start: 'HTTP/1.1 200 Connection Established', status: 200, request: false, connect: true }
  ]

  for (const { start, status, request, connect } of cases) {
    for (const upgrade of [false, true]) {
      for (const callbacks of [false, true]) {
        const label = `${start}, upgrade=${upgrade}, callbacks=${callbacks}`
        const expected = upgrade && (request || status === 101)
        const received = []
        const milo = setup({
          on_headers (parser, at, methodOrStatus, keepAlive, shouldUpgrade) {
            received.push({ methodOrStatus, shouldUpgrade: Boolean(shouldUpgrade) })
          }
        })
        const parser = milo.create()
        const message = Buffer.from(`${start}\r\n${upgrade ? 'Connection: upgrade\r\nUpgrade: h2c\r\n' : ''}\r\n`)
        const ptr = milo.alloc(message.length)
        try {
          milo.setShouldAutodetect(parser, false)
          milo.setIsRequest(parser, request)
          milo.setShouldSuspendAfterHeaders(parser, true)
          milo.setActiveEvents(parser, callbacks ? 0n : milo.EVENT_ACTIVE_ON_HEADERS)
          milo.setActiveCallbacks(parser, callbacks ? milo.CALLBACK_ACTIVE_ON_HEADERS : 0n)
          new Uint8Array(milo.memory.buffer, ptr, message.length).set(message)
          assert.equal(milo.parse(parser, ptr, message.length), message.length, label)
          assert.equal(milo.getErrorCode(parser), milo.ERROR_NONE, label)

          const methodOrStatus = request ? (connect ? milo.METHOD_CONNECT : milo.METHOD_POST) : status
          if (callbacks) {
            assert.deepEqual(received, [{ methodOrStatus, shouldUpgrade: expected }], label)
          } else {
            const fields = new DataView(milo.memory.buffer)
            const events = fields.getUint32(parser + milo.ParserFields.EVENTS, true)
            assert.equal(fields.getUint8(events), milo.EVENT_HEADERS, label)
            assert.equal(fields.getUint16(events + 5, true), methodOrStatus, label)
            assert.equal(fields.getUint8(events + 8), Number(expected), label)
          }

          // Supply CONNECT response context after parsing headers, before deciding the body framing.
          if (!request && connect) {
            milo.setIsConnect(parser, true)
          }
          milo.setShouldSuspendAfterHeaders(parser, false)
          milo.parse(parser, ptr, 0)
          assert.equal(milo.getErrorCode(parser), milo.ERROR_NONE, label)
          assert.equal(milo.getState(parser) === milo.STATE_TUNNEL, connect || expected, label)
        } finally {
          milo.destroy(parser)
          milo.dealloc(ptr, message.length)
        }
      }
    }
  }
})
