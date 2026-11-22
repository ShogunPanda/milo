import assert from 'node:assert/strict'
import test from 'node:test'
import { setup } from './helpers.js'

const request = Buffer.from('GET / HTTP/1.1\r\nHost: example.test\r\n\r\n')

test('callback controls do not change the batch', t => {
  let headers = 0
  let destroyResult = null
  const milo = setup({
    on_message_start (parser) {
      assert.equal(milo.parse(parser, ptr, request.length), 0)
      milo.reset(parser, false)
      milo.finish(parser)
      destroyResult = milo.destroy(parser)
      assert.equal(milo.getState(parser), milo.STATE_START)
      assert.equal(milo.getParsed(parser), BigInt(request.length))
      milo.pause(parser)
      assert.equal(milo.parse(other, ptr, request.length), request.length)
    },
    on_headers (parser) {
      assert.equal(milo.isPaused(parser), true)
      milo.resume(parser)
      headers++
    }
  })
  const parser = milo.create()
  const other = milo.create()
  const ptr = milo.alloc(request.length)
  new Uint8Array(milo.memory.buffer, ptr, request.length).set(request)
  t.after(() => {
    assert.equal(milo.destroy(parser), undefined)
    milo.destroy(other)
    milo.dealloc(ptr, request.length)
  })

  milo.setActiveCallbacks(parser, milo.CALLBACK_ACTIVE_ON_MESSAGE_START | milo.CALLBACK_ACTIVE_ON_HEADERS)
  assert.equal(milo.parse(parser, ptr, request.length), request.length)
  assert.equal(destroyResult, undefined)
  assert.equal(headers, 1)
  assert.equal(milo.isPaused(parser), false)
  milo.reset(parser, false)
  assert.equal(milo.getParsed(parser), 0n)
  milo.finish(parser)
  assert.equal(milo.getState(parser), milo.STATE_FINISH)
})

test('an exception handled inside a callback does not leave the parser guarded', t => {
  const failure = new Error('callback failed')
  let handled
  const milo = setup({
    on_headers () {
      try {
        throw failure
      } catch (error) {
        handled = error
      }
    }
  })
  const parser = milo.create()
  const ptr = milo.alloc(request.length)
  new Uint8Array(milo.memory.buffer, ptr, request.length).set(request)
  t.after(() => {
    milo.destroy(parser)
    milo.dealloc(ptr, request.length)
  })

  milo.setActiveCallbacks(parser, milo.CALLBACK_ACTIVE_ON_HEADERS)
  assert.equal(milo.parse(parser, ptr, request.length), request.length)
  assert.equal(handled, failure)
  milo.reset(parser, false)
  assert.equal(milo.getParsed(parser), 0n)
  milo.setActiveCallbacks(parser, 0n)
  assert.equal(milo.parse(parser, ptr, request.length), request.length)
})
