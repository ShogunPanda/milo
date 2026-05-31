import { readFile, writeFile } from 'node:fs/promises'
import { getBuildInfo } from './buildinfo.js'

async function prependVersionAndMethodMap () {
  const headerMatcher = 'namespace milo_parser {'

  const {
    version: { raw, major, minor, patch, prerelease },
    constants
  } = await getBuildInfo()

  // Create the method map, required by Node.js
  const methods = Object.entries(constants)
    .filter(p => p[0].startsWith('METHOD_'))
    .map(([k, v]) => [k.replace('METHOD_', ''), v])

  const updatedHeader = `
#define MILO_VERSION "${raw}"
#define MILO_VERSION_MAJOR ${major}
#define MILO_VERSION_MINOR ${minor}
#define MILO_VERSION_PATCH ${patch}
#define MILO_VERSION_PRERELEASE "${prerelease}"

#define MILO_METHODS_MAP(EACH) \\
${methods.map(([v, i]) => `  EACH(${i}, ${v}, ${v}) \\`).join('\n')}

namespace milo_parser {

struct Parser;
`.trim()

  // Replace the header with the new code
  return header.replace(/\n{3,}/g, '\n\n').replace(headerMatcher, updatedHeader)
}

// Read the file
const headerPath = new URL(`../dist/cpp/${process.argv[2]}/milo.h`, import.meta.url)
let header = await readFile(headerPath, 'utf-8')

// Apply modifications
header = await prependVersionAndMethodMap(header)

// Write the updated file
await writeFile(headerPath, header, 'utf-8')
