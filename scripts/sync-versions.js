#!/usr/bin/env node

import { execSync } from 'node:child_process'
import { readFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'

const folders = ['macros', 'parser', 'references/rust', 'parser/src/wasm']
const version = process.argv[2]

for (const folder of folders) {
  const path = resolve(import.meta.dirname, '..', folder)
  process.chdir(path)
  execSync(`cambi update ${version}`)
}

// Dependency of milo-macros
const mainCargo = resolve(import.meta.dirname, '../parser/Cargo.toml')
let mainCargoContent = await readFile(mainCargo, 'utf-8')
mainCargoContent = mainCargoContent.replace(/^(\s+milo-macros = \{ version = ")([^"]+)(")/m, `$1${version}$3`)
await writeFile(mainCargo, mainCargoContent)
