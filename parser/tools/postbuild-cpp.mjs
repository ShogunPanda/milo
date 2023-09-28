import { readFile, writeFile } from 'node:fs/promises'

async function prependVersionAndMethodMap() {
  const buildInfoPath = new URL('../target/buildinfo.json', import.meta.url)
  const headerMatcher = new RegExp(`^(?:namespace milo \{\n\n)$`, 'm')

  // Read the info file
  const {
    version: { major, minor, patch },
    constants
  } = JSON.parse(await readFile(buildInfoPath, 'utf-8'))

  // Create the method map, required by Node.js
  const methods = Object.entries(constants)
    .filter(p => p[0].startsWith('METHOD_'))
    .map(([k, v]) => [k.replace('METHOD_', ''), v])

  const updatedHeader = `
#define MILO_VERSION "${major}.${minor}.${patch}"
#define MILO_VERSION_MAJOR ${major}
#define MILO_VERSION_MINOR ${minor}
#define MILO_VERSION_PATCH ${patch}

#define MILO_METHODS_MAP(EACH) \\
${methods.map(([v, i]) => `  EACH(${i}, ${v}, ${v}) \\`).join('\n')}

namespace milo {

struct Parser;
`.trim()

  // Replace the header with the new code
  return header.replace(headerMatcher, updatedHeader)
}

function applyConfiguration() {
  const headerMathcer = new RegExp(`^(?:namespace milo \{\n\n)$`, 'm')
  const flags = ['DEBUG', 'ALL_CALLBACKS']
  const configuration = Object.fromEntries(process.argv[3].split(',').map(p => p.split(':')))

  for (const flag of flags) {
    header = header.replace(
      new RegExp(`constexpr static const bool ${flag} = (?:true|false);`),
      `constexpr static const bool ${flag} = ${configuration[flag]};`
    )
  }

  return header
}

// Read the file
const headerPath = new URL(`../dist/cpp/${process.argv[2]}/milo.h`, import.meta.url)
let header = await readFile(headerPath, 'utf-8')

// Apply modifications
header = await prependVersionAndMethodMap(header)
header = applyConfiguration(header)

// Write the updated file
await writeFile(headerPath, header, 'utf-8')
