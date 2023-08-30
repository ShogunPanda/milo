import { execSync } from 'node:child_process'
import { readFile, writeFile } from 'node:fs/promises'
import semver from 'semver'

const headerPath = new URL('../../dist/milo.h', import.meta.url)
const output = execSync('cargo metadata --format-version=1 --no-deps')
const parsed = JSON.parse(output)
const version = parsed.packages[0].version

const toReplace = `#define MILO_H`

const code = `
${toReplace}
#define MILO_VERSION "${version}"
#define MILO_VERSION_MAJOR ${semver.major(version)}
#define MILO_VERSION_MINOR ${semver.minor(version)}
#define MILO_VERSION_PATCH ${semver.patch(version)}
`.trim()

const header = await readFile(headerPath, 'utf-8')
await writeFile(headerPath, header.replace(new RegExp(`^(?:${toReplace})$`, 'm'), code), 'utf-8')
