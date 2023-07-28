import { load } from 'js-yaml'
import { readFile, writeFile } from 'node:fs/promises'

const headerPath = new URL('../../dist/milo.h', import.meta.url)

const methods = load(await readFile(new URL('../../../macros/src/methods.yml', import.meta.url)))

const toReplace = `namespace milo {`

const code = `
#define MILO_METHODS_MAP(EACH) \\
${methods.map((v, i) => `  EACH(${i}, ${v.replaceAll('-', '_')}, ${v.replaceAll('-', '_')}) \\`).join('\n')}


${toReplace}

struct Parser;

`.trim()

const header = await readFile(headerPath, 'utf-8')
await writeFile(headerPath, header.replace(new RegExp(`^(?:${toReplace})$`, 'm'), code), 'utf-8')
