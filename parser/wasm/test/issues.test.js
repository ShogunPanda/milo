import assert from 'node:assert/strict'
import { it } from 'node:test'
import { createParser, setup } from './helpers.js'

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
