// Exercise the generated packages, including their JavaScript bindings.
process.env.MILO_VARIANT ??= 'simd'
export const { setup } = await import(`../../../dist/wasm/release/package/src/${process.env.MILO_VARIANT}/index.js`)

export function createParser (t, setup) {
  const headers = []
  // Each test owns an instance, so its linear memory and input allocations are
  // isolated and become collectible together when the test finishes.
  const milo = setup({
    on_headers (parser, at, len) {
      headers.push({ parser, at, len })
    }
  })
  const parser = milo.create()
  milo.setActiveCallbacks(parser, milo.CALLBACK_ACTIVE_ALL)
  t.after(() => milo.destroy(parser))

  function parse (input) {
    const bytes = Buffer.from(input)
    const ptr = milo.alloc(bytes.length)
    // Allocation can grow memory and invalidate previously acquired views.
    new Uint8Array(milo.memory.buffer, ptr, bytes.length).set(bytes)
    return milo.parse(parser, ptr, bytes.length)
  }

  return { milo, parser, parse, headers }
}

// Match the HTTP fixture normalization in parser/tests/helpers/mod.rs.
export function http (input) {
  return input
    .trim()
    .replace(/^\s+/gm, '')
    .replaceAll('\n', '')
    .replaceAll('\\r', '\r')
    .replaceAll('\\n', '\n')
    .replaceAll('\\s', ' ')
}
