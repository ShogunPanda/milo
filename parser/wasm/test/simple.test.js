import assert from 'node:assert/strict'
import { it } from 'node:test'
import { simple } from './helpers.js'

function parse (milo, parser, input) {
  const bytes = Buffer.from(input)
  const ptr = milo.alloc(bytes.length)
  try {
    new Uint8Array(milo.memory.buffer, ptr, bytes.length).set(bytes)
    return milo.parse(parser, ptr, bytes.length)
  } finally {
    milo.dealloc(ptr, bytes.length)
  }
}

it('simple_span_collection_is_bounded_and_drainable', () => {
  assert.throws(() => simple({ maxSpans: -1 }), RangeError)
  assert.throws(() => simple({ maxSpans: 1.5 }), RangeError)

  const milo = simple({ maxSpans: 2 })
  const parser = milo.create()

  assert.equal(parse(milo, parser, 'GET / HTTP/1.1\r\n\r\n'), 18)
  assert.equal(milo.spans[parser].length, 2)
  assert.equal(milo.spanOverflow[parser], true)

  const first = milo.drain(parser)
  assert.equal(first.spans.length, 2)
  assert.equal(first.truncated, true)
  assert.deepEqual(milo.spans[parser], [])
  assert.equal(milo.spanOverflow[parser], false)

  assert.equal(parse(milo, parser, 'GET / HTTP/1.1\r\n\r\n'), 18)
  const second = milo.drain(parser)
  assert.equal(second.spans.length, 2)
  assert.equal(second.truncated, true)

  milo.destroy(parser)
  assert.equal(parser in milo.spans, false)
  assert.equal(parser in milo.spanOverflow, false)
})
