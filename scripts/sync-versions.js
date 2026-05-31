#!/usr/bin/env node

import { execSync } from 'node:child_process'
import { resolve } from 'node:path'

const folders = ['macros', 'parser', 'references/rust', 'parser/src/wasm']
const version = process.argv[2]

for (const folder of folders) {
  const path = resolve(import.meta.dirname, '..', folder)
  process.chdir(path)
  execSync(`cambi update ${version}`)
}
