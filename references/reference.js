import { deepStrictEqual, strictEqual } from 'node:assert'
import { spawnSync } from 'node:child_process'
import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { test } from 'node:test'

function parseNDJSON (raw) {
  return raw
    .trim()
    .replaceAll(/\n+\s*[-]+\n+/gm, '\n')
    .replaceAll(/^\s+/gm, '')
    .trim()
    .split('\n')
    .map(i => JSON.parse(i.trim()))
}

async function verifyReadme () {
  const expected = await readFile(resolve(import.meta.dirname, './fixtures/readme.txt'), 'utf-8')

  let path
  switch (process.env.VARIANT) {
    case 'rust':
      path = resolve(import.meta.dirname, '../tmp/references/rust/release/readme')
      break
    case 'cpp':
      path = resolve(import.meta.dirname, '../tmp/references/cpp/readme')
      break
    case 'wasm':
      path = resolve(import.meta.dirname, './wasm/src/readme.js')
      break
    default:
      throw process.env.VARIANT
        ? new Error(`Unknown variant: ${process.env.VARIANT}`)
        : new Error('VARIANT environment variable not set')
  }

  const actual = spawnSync(path).stdout.toString('utf-8').trim()

  deepStrictEqual(actual, expected)
}

async function verifyOutput (profile) {
  const expected = await readFile(resolve(import.meta.dirname, `./fixtures/${profile}.jsonl`), 'utf-8')
  const expectedLines = parseNDJSON(expected)

  let path
  switch (process.env.VARIANT) {
    case 'rust':
      path = resolve(import.meta.dirname, `../tmp/references/rust/${profile}/reference`)
      break
    case 'cpp':
      path = resolve(import.meta.dirname, `../tmp/references/cpp/${profile}`)
      break
    case 'wasm':
      process.env.WASM_PROFILE = profile
      path = resolve(import.meta.dirname, './wasm/src/reference.js')
      break
    default:
      throw process.env.VARIANT
        ? new Error(`Unknown variant: ${process.env.VARIANT}`)
        : new Error('VARIANT environment variable not set')
  }

  const actual = spawnSync(path).stdout.toString('utf-8')
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

test('readme', t => {
  return verifyReadme()
})

test('debug', t => {
  return verifyOutput('debug')
})

test('release', t => {
  return verifyOutput('release')
})
