import { setup } from '@perseveranza-pets/milo'

/*
  Milo works using callbacks.

  All callbacks have the same signature, which characterizes the payload:

    * The current parser
    * from: The payload offset.
    * size: The payload length.

  The payload parameters above are relative to the last data sent to the milo.parse method.

  If the current callback has no payload, both values are set to 0.

  The callbacks must be provided using setup and are named in snake case.
*/
const milo = setup({
  on_data(p, from, size) {
    console.log(`Pos=${milo.getPosition(p)} Body: ${message.slice(from, from + size).toString()}`)
  }
})

// Prepare a message to parse.
const message = Buffer.from('HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc')

// Allocate a memory in the WebAssembly space. This speeds up data copying to the WebAssembly layer.
const ptr = milo.alloc(message.length)

// Create a buffer we can use normally.
const buffer = Buffer.from(milo.memory.buffer, ptr, message.length)

// Create the parser.
const parser = milo.create()

// Now perform the main parsing using milo.parse. The method returns the number of consumed characters.
buffer.set(message, 0)
milo.parse(parser, ptr, message.length)

// Cleanup used resources.
milo.destroy(parser)
milo.dealloc(ptr, message.length)
