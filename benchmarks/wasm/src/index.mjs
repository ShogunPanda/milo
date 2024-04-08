import { setup } from '@perseveranza-pets/milo'
import { readFile } from 'node:fs/promises'

async function loadMessage(name) {
  const raw = await readFile(new URL(`../../fixtures/${name}.txt`, import.meta.url), 'utf-8')

  return raw.trim().replaceAll('\n', '').replaceAll('\\r\\n', '\r\n')
}

function formatNumber(num, precision) {
  return num
    .toLocaleString('en-US', {
      minimumFractionDigits: precision,
      maximumFractionDigits: precision,
      useGrouping: 'always'
    })
    .replaceAll(',', '_')
}

const samples = ['seanmonstar_httparse', 'nodejs_http_parser', 'undici']
const milo = setup()

for (const name of samples) {
  const payload = await loadMessage(name)
  const len = payload.length
  const iterations = 2 ** 33 / len
  const total = iterations * len

  const parser = milo.create()
  const ptr = milo.alloc(len)
  const buffer = Buffer.from(milo.memory.buffer, ptr, len)
  buffer.set(Buffer.from(payload))

  const start = process.hrtime.bigint()
  for (let i = 0; i < iterations; i++) {
    milo.parse(parser, ptr, len)
  }

  milo.destroy(parser)
  milo.dealloc(ptr, len)

  const time = Number(process.hrtime.bigint() - start) / 1e9
  const bw = total / time

  const label = name.padStart(21, ' ')
  const samples = formatNumber(iterations, 0).padStart(12)
  const size = formatNumber(total / (1024 * 1024), 2).padStart(8)
  const speed = formatNumber(bw / (1024 * 1024), 2).padStart(10)
  const throughtput = formatNumber(iterations / time, 2).padStart(10)
  const duration = formatNumber(time, 2).padStart(6)

  console.log(`${label} | ${samples} samples | ${size} MB | ${speed} MB/s | ${throughtput} ops/sec | ${duration} s`)
}
