#!/usr/bin/env node

import { info, load } from './parser.mjs'

export async function main() {
  const [milo, parser, context] = await load()
  const ptr = milo.alloc(1000)

  let request1 = 'GET / HTTP/1.1\r\n\r\n'
  let request2 =
    'HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: x-trailer\r\n\r\nc;need=love\r\nhello world!\r\n0\r\nX-Trailer: value\r\n\r\n'

  const buffer = Buffer.from(milo.memory.buffer, ptr, 1000)
  context.input = buffer
  buffer.set(Buffer.from(request1), 0)

  let consumed = milo.parse(parser, ptr, request1.length)
  info(
    `{ "pos": ${milo.getPosition(parser)}, "consumed": ${consumed}, "state": "${milo.States[milo.getState(parser)]}" }`
  )

  info('\n------------------------------------------------------------------------------------------\n')

  buffer.set(Buffer.from(request2), 0)
  consumed = milo.parse(parser, ptr, request2.length)
  info(
    `{ "pos": ${milo.getPosition(parser)}, "consumed": ${consumed}, "state": "${milo.States[milo.getState(parser)]}" }`
  )
}

await main()
