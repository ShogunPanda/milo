/* eslint-disable no-unused-vars */

import { setup } from '@perseveranza-pets/milo'

function onMethod() {
  console.log('CALLBACK: onMethod')
}

function onUrlValid() {
  console.log('CALLBACK: onUrlValid')
}

function onUrlThrow() {
  console.log('CALLBACK: onUrlThrow')
  throw new Error('WTF')
}

function onUrlJSError(parser, from, size) {
  console.log('CALLBACK: onUrlJSError')
  // eslint-disable-next-line no-undef
  a = b
}

// TODO@PI: Support QUERY method (https://datatracker.ietf.org/doc/draft-ietf-httpbis-safe-method-w-body/)
export async function main() {
  const milo = setup({ on_method: onMethod, on_url: onUrlJSError })
  const parser = milo.create()

  const ptr = milo.alloc(100)
  const buffer = Buffer.from(milo.memory.buffer, ptr, 100)

  const message = 'GET / HTTP/1.1\r\n\r\n'
  buffer.set(Buffer.from(message), 0)

  try {
    const consumed = milo.parse(parser, ptr, message.length)
    const state = milo.States[milo.getState(parser)]

    console.log(
      'AFTER 1:',
      JSON.stringify(
        {
          pos: milo.getPosition(parser),
          consumed,
          state,
          errorCode: milo.Errors[milo.getErrorCode(parser)],
          errorDescription: milo.getErrorDescription(parser)
        },
        null,
        2
      )
    )
  } catch (e) {
    console.log('ERROR 1', e)
  }

  milo.reset(parser, true)
  milo.clear()

  try {
    const consumed = milo.parse(parser, ptr, message.length)
    const state = milo.States[milo.getState(parser)]

    console.log(
      'AFTER 2:',
      JSON.stringify(
        {
          pos: milo.getPosition(parser),
          consumed,
          state,
          errorCode: milo.Errors[milo.getErrorCode(parser)],
          errorDescription: milo.getErrorDescription(parser)
        },
        null,
        2
      )
    )
  } catch (e) {
    console.log('ERROR 2', e)
  }

  milo.destroy(parser)
  milo.dealloc(ptr, 100)

  console.log('DONE')
}

await main()
