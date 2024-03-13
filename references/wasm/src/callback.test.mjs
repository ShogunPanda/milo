function onMethod1(parser, from, size) {
  return 1
}

function onMethod2(parser, from, size) {
  throw new Error('WTF')
}

function onMethod3(parser, from, size) {
  a = b
}

export async function main() {
  const milo = await import(`../lib/${process.env.CONFIGURATION ?? process.argv[2]}/milo.js`)
  const parser = milo.Parser.create()

  // parser.setOnError(onError.bind(milo, parser))
  parser.setOnMethod(onMethod3.bind(milo, parser))

  const request = Buffer.from('GET / HTTP/1.1\r\n\r\n')

  let consumed = parser.parse(request.subarray(0, 65535), 65535)
  consumed = parser.parse(request.subarray(65535), request.length - 65535)
  const state = milo.States[parser.state]
  console.log(
    `{ "pos": ${parser.position}, "consumed": ${consumed}, "state": "${state}", "description": "${parser.errorDescription}" }`
  )
  console.log(parser.callbackError)
}

await main()
