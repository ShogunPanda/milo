import { readFile } from 'node:fs/promises'

const [version] = process.argv.slice(2)
if (!version) {
  throw new Error('Usage: node scripts/release-notes.js <version>')
}

const lines = (await readFile('CHANGELOG.md', 'utf8')).split(/\r?\n/)
const start = lines.findIndex(line => {
  const parts = line.trimEnd().split(' / ')
  return parts.length === 2 && parts[0].startsWith('### ') && parts[1] === version
})
const end = lines.findIndex((line, index) => index > start && line.startsWith('### '))
const notes = start === -1 ? '' : lines.slice(start + 1, end === -1 ? lines.length : end).join('\n').trim()
if (!notes) {
  throw new Error(`No changelog notes for version ${version}`)
}

console.log(notes)
