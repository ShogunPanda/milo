function onMethod1(parser, from, size) {
  return 'NO'
}

function onMethod2(parser, from, size) {
  throw new Error('WTF')
}

function onMethod3(parser, from, size) {
  a = b
}

export async function main() {
  const milo = await import(`../lib/${process.env.CONFIGURATION ?? process.argv[2]}/milo.js`)
  const parser = milo.create()

  const ptr = milo.alloc(100)
  const buffer = Buffer.from(milo.memory.buffer, ptr, 100)

  // milo.setOnError(parser, onError.bind(milo, parser))
  milo.setOnMethod(parser, onMethod1.bind(milo, parser))
  milo.setOnHeaders(parser, onMethod1.bind(milo, parser))

  const message = 'GET / HTTP/1.1\r\n\r\n'
  buffer.set(Buffer.from(message), 0)

  const consumed = milo.parse(parser, ptr, message.length)

  const state = milo.States[milo.getState(parser)]
  console.log(parser)
  console.log(
    JSON.stringify(
      {
        pos: milo.getPosition(parser),
        consumed,
        state,
        errorCode: milo.getErrorCodeString(parser),
        errorDescription: milo.getErrorDescriptionString(parser)
      },
      null,
      2
    ),
    milo.getCallbackError(parser)
  )

  milo.free(ptr, 100)
  milo.destroy(parser)
}

await main()
