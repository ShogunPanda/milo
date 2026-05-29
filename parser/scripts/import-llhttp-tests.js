import { execFile } from 'node:child_process'
import { glob, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { join, relative } from 'node:path'
import { promisify } from 'node:util'
import remarkParse from 'remark-parse'
import { unified } from 'unified'
import YAML from 'yaml'

const fixturePrefix = 'tests/fixtures/llhttp'
const execFileAsync = promisify(execFile)

function fail (message) {
  console.error(message)
  process.exit(1)
}

function cleanHeading (title) {
  return title.replace(/`/g, '').trim()
}

function decodeHtmlEntities (value) {
  return value.replace(/&(?:quot|apos|amp|lt|gt|#(\d+)|#x([0-9a-fA-F]+));/g, (match, dec, hex) => {
    switch (match) {
      case '&quot;':
        return '"'
      case '&apos;':
        return "'"
      case '&amp;':
        return '&'
      case '&lt;':
        return '<'
      case '&gt;':
        return '>'
      default:
        if (dec) {
          return String.fromCodePoint(Number(dec))
        }

        if (hex) {
          return String.fromCodePoint(Number.parseInt(hex, 16))
        }

        return match
    }
  })
}

function parseHtmlMeta (value) {
  const match = value.match(/<!--\s*meta=(.*?)\s*-->/s) || value.match(/^\s*meta=(.*?)\s*$/s)
  if (!match) {
    return null
  }

  const meta = JSON.parse(decodeHtmlEntities(match[1].trim()))
  if (!meta || typeof meta !== 'object' || Array.isArray(meta) || Object.keys(meta).length === 0) {
    return null
  }

  return meta
}

function hasMeta (value) {
  return value && typeof value === 'object' && !Array.isArray(value) && Object.keys(value).length > 0
}

function stringifyFixture (fixture) {
  const yamlBody = YAML.stringify(fixture, {
    lineWidth: 0,
    defaultStringType: 'QUOTE_SINGLE',
    defaultKeyType: 'PLAIN'
  })

  return `---\n${yamlBody}`
}

function normalizeFixtureForComparison (value) {
  if (Array.isArray(value)) {
    return value.map(normalizeFixtureForComparison)
  }

  if (value && typeof value === 'object') {
    const normalized = {}

    for (const key of Object.keys(value).sort()) {
      if (key === 'checked') {
        continue
      }

      normalized[key] = normalizeFixtureForComparison(value[key])
    }

    return normalized
  }

  return value
}

async function processSection (llhttpRoot, outputRoot, fixtureRoot, section, seenFiles) {
  const source = join(llhttpRoot, 'test', section)
  const cases = []
  const usedNames = new Set()

  // Find markdown files
  let files = []
  for await (const entry of glob('**/*.md', { cwd: source })) {
    files.push(join(source, entry))
  }

  files = files.sort((a, b) => a.localeCompare(b))

  // Process each file
  for (const file of files) {
    const raw = await readFile(file, 'utf8')
    const sourcePath = relative(llhttpRoot, file).replace(/\\/g, '/')
    // Parse llhttp markdown sections into test cases.
    const markdownParser = unified().use(remarkParse)
    const tree = markdownParser.parse(raw)
    const parsed = []

    let currentSection
    let currentCase = null

    const flushCase = () => {
      if (currentCase && currentCase.http != null && currentCase.log != null) {
        parsed.push({
          titles: currentCase.titles,
          source: currentCase.source,
          meta: currentCase.meta,
          http: currentCase.http,
          log: currentCase.log
        })
      }

      currentCase = null
    }

    for (const node of tree.children) {
      if (node.type === 'heading' && node.depth === 2) {
        flushCase()
        const headingText = node.children
          .filter(child => child.type === 'text' || child.type === 'inlineCode')
          .map(child => child.value)
          .join('')
        currentSection = cleanHeading(headingText)
        currentCase = {
          titles: [currentSection],
          source: {
            path: sourcePath,
            line: node.position?.start?.line ?? 1
          },
          meta: null,
          http: null,
          log: null
        }

        continue
      }

      if (node.type === 'heading' && node.depth === 3) {
        flushCase()
        const headingText = node.children
          .filter(child => child.type === 'text' || child.type === 'inlineCode')
          .map(child => child.value)
          .join('')
        const cleanedHeading = cleanHeading(headingText)

        if (!currentSection) {
          currentCase = {
            titles: [cleanedHeading],
            source: {
              path: sourcePath,
              line: node.position?.start?.line ?? 1
            },
            meta: null,
            http: null,
            log: null
          }
        } else {
          currentCase = {
            titles: [currentSection, cleanedHeading],
            source: {
              path: sourcePath,
              line: node.position?.start?.line ?? 1
            },
            meta: null,
            http: null,
            log: null
          }
        }

        continue
      }

      if (node.type === 'html' && currentCase && currentCase.http == null) {
        const meta = parseHtmlMeta(node.value)

        if (meta) {
          currentCase.meta = { ...(currentCase.meta ?? {}), ...meta }
        }

        continue
      }

      if (node.type === 'code' && currentCase) {
        const codeLang = (node.lang || '').toLowerCase()

        if (codeLang === 'http') {
          currentCase.http = node.value
        } else if (codeLang === 'log') {
          currentCase.log = node.value
        }
      }
    }

    flushCase()

    for (const item of parsed) {
      if (!item.http || !item.log) {
        continue
      }

      // Build deterministic fixture file name from titles.
      const name = item.titles
        .map(part =>
          cleanHeading(part)
            .normalize('NFKD')
            .toLowerCase()
            .replace(/[^a-z0-9\s-]/g, ' ')
            .trim()
            .replace(/[\s_]+/g, '-')
            .replace(/-+/g, '-')
            .replace(/^-+|-+$/g, ''))
        .filter(Boolean)
        .join('-')
      let fileName = `${name || 'test'}.yml`

      if (!usedNames.has(fileName)) {
        usedNames.add(fileName)
      } else {
        let counter = 2
        while (usedNames.has(`${name}-${counter}.yml`)) {
          counter += 1
        }

        fileName = `${name}-${counter}.yml`
        usedNames.add(fileName)
      }

      const input = item.http
      const [prefix, child] = item.titles
      // Keep display names aligned with existing fixture format.
      const titleCase = value =>
        value
          .split(/\s+/)
          .filter(Boolean)
          .map(part => `${part[0].toUpperCase()}${part.slice(1)}`)
          .join(' ')
      const fixture = {
        path: join(fixturePrefix, section, fileName),
        name: child ? `${cleanHeading(prefix)} / ${titleCase(cleanHeading(child))}` : titleCase(cleanHeading(prefix)),
        checked: false,
        source: item.source,
        ...(hasMeta(item.meta) ? { meta: item.meta } : {}),
        input: input.split('\n'),
        llhttp: item.log.split('\n')
      }

      cases.push({
        path: fileName,
        fixture
      })
    }
  }

  const targetDir = join(fixtureRoot, `${section}s`)
  await mkdir(targetDir, { recursive: true })
  const tempFixturePath = join(targetDir, `.import-llhttp-${section}-temp.yml`)

  let i = 0
  for (const item of cases) {
    console.log(`Processing ${section} case ${++i}/${cases.length}: ${item.path}`)
    const filePath = join(targetDir, item.path)
    seenFiles.add(join(`${section}s`, item.path).replace(/\\/g, '/'))
    const initialContent = stringifyFixture({
      ...item.fixture,
      output: []
    })

    await writeFile(tempFixturePath, initialContent.endsWith('\n') ? initialContent : `${initialContent}\n`, 'utf8')

    // Run generator using a temporary fixture to compute output.
    const fileArg = relative(outputRoot, tempFixturePath).replace(/\\/g, '/')
    let stdout

    try {
      const result = await execFileAsync('cargo', ['run', '--example', 'llhttp', '--', '--generate', fileArg], {
        cwd: outputRoot,
        maxBuffer: 10 * 1024 * 1024
      })

      stdout = result.stdout
    } catch (error) {
      const details = typeof error?.stderr === 'string' && error.stderr.trim() ? `\n${error.stderr.trim()}` : ''
      fail(`Failed to run cargo generator for ${fileArg}${details}`)
    }

    const markerIndex = stdout.indexOf('---')
    if (markerIndex === -1) {
      fail(`Failed to parse cargo output for ${fileArg}: missing YAML marker`)
    }

    const yamlSnippet = stdout.slice(markerIndex + 3).trim()
    if (!yamlSnippet) {
      fail(`Failed to parse cargo output for ${fileArg}: empty YAML snippet`)
    }

    const output = YAML.parse(yamlSnippet)
    const finalFixture = {
      ...item.fixture,
      output
    }

    // Skip writes when only comments/checked differ semantically.
    let skipOverwrite = false

    try {
      const raw = await readFile(filePath, 'utf8')

      try {
        const existingFixture = YAML.parse(raw)
        skipOverwrite =
          JSON.stringify(normalizeFixtureForComparison(existingFixture)) ===
          JSON.stringify(normalizeFixtureForComparison(finalFixture))
      } catch {
        skipOverwrite = false
      }
    } catch (error) {
      if (!error || error.code !== 'ENOENT') {
        throw error
      }
    }

    if (skipOverwrite) {
      continue
    }

    const finalContent = stringifyFixture(finalFixture)

    await writeFile(filePath, finalContent.endsWith('\n') ? finalContent : `${finalContent}\n`, 'utf8')
  }

  await rm(tempFixturePath, { force: true })
}

async function main () {
  const [, , llhttpRoot, outputRoot] = process.argv
  const fixtureRoot = join(outputRoot, fixturePrefix)

  if (!llhttpRoot || !outputRoot) {
    console.log('Usage: node parser/scripts/import-llhttp-tests.js <llhttp-root> <output-root>')
    process.exit(0)
  }

  await mkdir(fixtureRoot, { recursive: true })
  // Snapshot fixtures that existed before import starts.
  const existingFiles = new Set()

  for (const section of ['requests', 'responses']) {
    const sectionRoot = join(fixtureRoot, section)

    try {
      for await (const entry of glob('**/*.yml', { cwd: sectionRoot })) {
        existingFiles.add(join(section, entry).replace(/\\/g, '/'))
      }
    } catch {
      // Section folder may not exist yet.
    }
  }

  const seenFiles = new Set()

  await processSection(llhttpRoot, outputRoot, fixtureRoot, 'request', seenFiles)
  await processSection(llhttpRoot, outputRoot, fixtureRoot, 'response', seenFiles)

  for (const file of existingFiles) {
    if (seenFiles.has(file)) {
      continue
    }

    await rm(join(fixtureRoot, file), { force: true })
    console.log(`Removed stale fixture: ${file}`)
  }
}

main()
