import assert from 'node:assert/strict'
import { it } from 'node:test'
import { setup } from './helpers.js'

function allocate (milo, input) {
  const bytes = Buffer.from(input)
  const ptr = milo.alloc(bytes.length)
  new Uint8Array(milo.memory.buffer, ptr, bytes.length).set(bytes)
  return { bytes, ptr }
}

it('callback_reentry_and_mutation_are_rejected', t => {
  let nestedConsumed
  let mutationRejected = false
  let messageStartCalled = false

  const milo = setup({
    on_request (parser) {
      const state = milo.getState(parser)
      const parsed = milo.getParsed(parser)
      const errorCode = milo.getErrorCode(parser)

      nestedConsumed = milo.parse(parser, 0, 0)
      milo.pause(parser)
      milo.reset(parser, false)
      milo.clear(parser)
      milo.complete(parser)
      milo.finish(parser)
      milo.fail(parser, milo.ERROR_UNEXPECTED_CHARACTER, 'callback mutation')
      milo.setIsRequest(parser, false)

      mutationRejected =
        milo.getState(parser) === state &&
        milo.getParsed(parser) === parsed &&
        milo.getErrorCode(parser) === errorCode &&
        !milo.isPaused(parser) &&
        milo.isRequest(parser)

      // The current callback batch uses an immutable activation snapshot.
      milo.setActiveCallbacks(parser, 0n)
    },
    on_message_start () {
      messageStartCalled = true
    }
  })

  const parser = milo.create()
  t.after(() => milo.destroy(parser))
  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, true)
  milo.setActiveCallbacks(parser, milo.CALLBACK_ACTIVE_ON_REQUEST | milo.CALLBACK_ACTIVE_ON_MESSAGE_START)
  const { bytes, ptr } = allocate(milo, 'GET / HTTP/1.1\r\n\r\n')
  t.after(() => milo.dealloc(ptr, bytes.length))

  assert.equal(milo.parse(parser, ptr, bytes.length), bytes.length)
  assert.equal(nestedConsumed, 0)
  assert.equal(mutationRejected, true)
  assert.equal(messageStartCalled, true)
  assert.equal(milo.getState(parser), milo.STATE_START)
  assert.equal(milo.getErrorCode(parser), milo.ERROR_NONE)
})

it('callback_exceptions_are_rethrown_after_replay_cleanup', t => {
  const expected = new Error('callback failed')
  const milo = setup({
    on_request () {
      throw expected
    }
  })

  const parser = milo.create()
  t.after(() => milo.destroy(parser))
  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, true)
  milo.setActiveCallbacks(parser, milo.CALLBACK_ACTIVE_ON_REQUEST)
  const { bytes, ptr } = allocate(milo, 'GET / HTTP/1.1\r\n\r\n')
  t.after(() => milo.dealloc(ptr, bytes.length))

  assert.throws(
    () => milo.parse(parser, ptr, bytes.length),
    error => error === expected
  )

  // The replay guard was cleared before the JavaScript exception was rethrown.
  milo.setActiveCallbacks(parser, 0n)
  assert.equal(milo.parse(parser, ptr, bytes.length), bytes.length)
})

it('callback_destruction_is_deferred_and_stops_replay', t => {
  let callbackAfterDestroy = false

  const milo = setup({
    on_request (parser) {
      milo.destroy(parser)
    },
    on_message_start () {
      callbackAfterDestroy = true
    }
  })

  const parser = milo.create()
  milo.setShouldAutodetect(parser, false)
  milo.setIsRequest(parser, true)
  milo.setActiveCallbacks(parser, milo.CALLBACK_ACTIVE_ON_REQUEST | milo.CALLBACK_ACTIVE_ON_MESSAGE_START)
  const { bytes, ptr } = allocate(milo, 'GET / HTTP/1.1\r\n\r\n')
  t.after(() => milo.dealloc(ptr, bytes.length))

  assert.equal(milo.parse(parser, ptr, bytes.length), bytes.length)
  assert.equal(callbackAfterDestroy, false)
})
