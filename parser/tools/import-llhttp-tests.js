import { execFile } from 'node:child_process'
import { glob, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { join, relative } from 'node:path'
import { promisify } from 'node:util'
import remarkParse from 'remark-parse'
import { unified } from 'unified'
import YAML from 'yaml'

const fixturePrefix = 'tests/fixtures/llhttp'
const execFileAsync = promisify(execFile)

function fail(message) {
  console.error(message)
  process.exit(1)
}

function cleanHeading(title) {
  return title.replace(/`/g, '').trim()
}

function titleCase(value) {
  return value
    .split(/\s+/)
    .filter(Boolean)
    .map(part => `${part[0].toUpperCase()}${part.slice(1)}`)
    .join(' ')
}

function slugify(value) {
  return value
    .normalize('NFKD')
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, ' ')
    .trim()
    .replace(/[\s_]+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-+|-+$/g, '')
}

function formatDisplayName(parts) {
  const [prefix, child] = parts
  if (child) {
    return `${cleanHeading(prefix)} / ${titleCase(cleanHeading(child))}`
  }

  return titleCase(cleanHeading(prefix))
}

function formatFileName(parts, used) {
  const name = parts
    .map(part => slugify(cleanHeading(part)))
    .filter(Boolean)
    .join('-')
  let fileName = `${name || 'test'}.yml`

  if (!used.has(fileName)) {
    used.add(fileName)
    return fileName
  }

  let counter = 2
  while (used.has(`${name}-${counter}.yml`)) {
    counter += 1
  }

  fileName = `${name}-${counter}.yml`
  used.add(fileName)

  return fileName
}

function parseMarkdown(rawText, sourcePath) {
  const markdownParser = unified().use(remarkParse)
  const tree = markdownParser.parse(rawText)
  const tests = []

  let currentSection
  let currentCase = null

  function flushCase() {
    if (currentCase && currentCase.http != null && currentCase.log != null) {
      tests.push({
        titles: currentCase.titles,
        source: currentCase.source,
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
          http: null,
          log: null
        }
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

      continue
    }
  }

  flushCase()

  return tests
}

function stringifyFixture(fixture) {
  const yamlBody = YAML.stringify(fixture, {
    lineWidth: 0,
    defaultStringType: 'QUOTE_SINGLE',
    defaultKeyType: 'PLAIN'
  })

  return `---\n${yamlBody}`
}

async function generateOutput(outputRoot, section, filePath) {
  const fileArg = relative(outputRoot, filePath).replace(/\\/g, '/')
  let stdout

  try {
    const result = await execFileAsync(
      'cargo',
      ['run', '--example', 'llhttp', '--', '--generate', section + 's', fileArg],
      {
        cwd: outputRoot,
        maxBuffer: 10 * 1024 * 1024
      }
    )

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

  return YAML.parse(yamlSnippet)
}

async function processSection(llhttpRoot, outputRoot, fixtureRoot, section) {
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
    const parsed = parseMarkdown(raw, sourcePath)

    for (const item of parsed) {
      if (!item.http || !item.log) {
        continue
      }

      const fileName = formatFileName(item.titles, usedNames)

      const input = item.http
      const fixture = {
        path: join(fixturePrefix, section, fileName),
        name: formatDisplayName(item.titles),
        checked: false,
        source: item.source,
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

  let i = 0
  for (const item of cases) {
    console.log(`Processing ${section} case ${++i}/${cases.length}: ${item.path}`)
    const filePath = join(targetDir, item.path)
    const initialContent = stringifyFixture({
      ...item.fixture,
      output: []
    })

    await writeFile(filePath, initialContent.endsWith('\n') ? initialContent : `${initialContent}\n`, 'utf8')

    const output = await generateOutput(outputRoot, section, filePath)
    const finalContent = stringifyFixture({
      ...item.fixture,
      output
    })

    await writeFile(filePath, finalContent.endsWith('\n') ? finalContent : `${finalContent}\n`, 'utf8')
  }
}

async function main() {
  const [, , llhttpRoot, outputRoot] = process.argv
  const fixtureRoot = join(outputRoot, fixturePrefix)

  if (!llhttpRoot || !outputRoot) {
    console.log('Usage: node parser/tools/import-llhttp-tests.js <llhttp-root> <output-root>')
    process.exit(0)
  }

  await rm(fixtureRoot, { recursive: true, force: true })
  await mkdir(fixtureRoot, { recursive: true })

  await processSection(llhttpRoot, outputRoot, fixtureRoot, 'request')
  await processSection(llhttpRoot, outputRoot, fixtureRoot, 'response')
}

main()
