/* eslint-disable no-unused-vars */

import { milo } from '../../../parser/dist/wasm/debug/index.js'

function onMethodValid(parser, from, size) {
  console.log('CALLBACK: onMethodValid')
}

function onMethodThrow(parser, from, size) {
  console.log('CALLBACK: onMethodThrow')
  throw new Error('WTF')
}

function onMethodJSError(parser, from, size) {
  console.log('CALLBACK: onMethodJSError')

  // eslint-disable-next-line no-undef
  a = b
}

export async function main() {
  const parser = milo.create()

  const ptr = milo.alloc(100)
  const buffer = Buffer.from(milo.memory.buffer, ptr, 100)

  milo.setOnMethod(parser, onMethodJSError.bind(milo, parser))

  const message = 'GET / HTTP/1.1\r\n\r\n'
  buffer.set(Buffer.from(message), 0)

  const consumed = milo.parse(parser, ptr, message.length)

  const state = milo.States[milo.getState(parser)]
  console.log(
    'STATE:',
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
  if (milo.getErrorCode(parser) === milo.ERROR_CALLBACK_ERROR) {
    console.log('CALLBACK ERROR:', milo.getCallbackError(parser))
  }

  milo.destroy(parser)
  milo.dealloc(ptr, 100)
}

await main()
