import { deepStrictEqual, strictEqual } from 'node:assert'
import { spawnSync } from 'node:child_process'
import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'

function parseNDJSON(raw) {
  return raw
    .trim()
    .replaceAll(/\n+\s*[-]+\n+/gm, '\n')
    .replaceAll(/^\s+/gm, '')
    .trim()
    .split('\n')
    .map(i => JSON.parse(i.trim()))
}

async function verifyOutput(executable) {
  const expected = await readFile(fileURLToPath(new URL(`./fixtures/${executable}.jsonl`, import.meta.url)), 'utf-8')
  const expectedLines = parseNDJSON(expected)

  const actual = spawnSync(resolve(process.cwd(), 'dist', `reference-${executable}`)).stdout.toString('utf-8')
  const actualLines = parseNDJSON(actual)

  strictEqual(actualLines.length, expectedLines.length, 'Output length differs')
  for (let i = 0; i < expectedLines.length; i++) {
    deepStrictEqual(
      actualLines[i],
      expectedLines[i],
      `Line ${i + 1} differs.\nExpected: ${JSON.stringify(expectedLines[i])}\n     Got: ${JSON.stringify(
        actualLines[i]
      )}`
    )
  }
}

test('debug', t => {
  return verifyOutput('debug')
})

test('release', t => {
  return verifyOutput('release')
})
