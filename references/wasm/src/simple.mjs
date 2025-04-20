import { simple as milo } from '@perseveranza-pets/milo'

const message = Buffer.from('HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc')
const ptr = milo.alloc(message.length)
const parser = milo.create()

const buffer = Buffer.from(milo.memory.buffer, ptr, message.length)
buffer.set(message, 0)
milo.parse(parser, ptr, message.length)

for (const [type, at, len] of milo.spans[parser]) {
  console.log(`[${at.toString().padStart(3, ' ')}, ${len.toString().padStart(3, ' ')}] -> ${milo.Callbacks[type]}`)
}

milo.destroy(parser)
milo.dealloc(ptr, message.length)
