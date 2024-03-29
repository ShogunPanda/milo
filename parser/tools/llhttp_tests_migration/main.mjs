#!/usr/bin/env node

import { spawn } from 'node:child_process'
import { readFile } from 'node:fs/promises'
import { isAbsolute, resolve } from 'node:path'
import remarkParse from 'remark-parse'
import remarkStringify from 'remark-stringify'
import { unified } from 'unified'

function fatal(msg, node) {
  if (node) {
    msg += ` (line ${node.position.start.line}, column ${node.position.start.column})`
  }

  console.error(`\x1b[31m${msg}\x1b[0m`)
  process.exit(1)
}

function prepareTest() {
  return { title: '', input: '', output: '' }
}

async function serializeText(node) {
  const md = await unified().use(remarkStringify).stringify(node)

  return md.trim().replace(/^#+\s+/, '')
}

async function main() {
  if (process.argv.length < 3) {
    fatal(`Usage: ${process.argv[1]} [FILE]`)
  }

  // Parse the file using Markdown
  const relativeFile = process.argv[2]
  const prefix = process.argv[3]
  const file = isAbsolute(relativeFile) ? relativeFile : resolve(process.cwd(), relativeFile)

  const parsed = await unified()
    .use(remarkParse)
    .parse(await readFile(file, 'utf-8'))

  let title = ''
  const tests = []
  let lastTest = prepareTest()

  // For each node
  for (const node of parsed.children) {
    switch (node.type) {
      case 'heading':
        // Suite title
        if (node.depth === 1) {
          if (title.length) {
            fatal('Suite tite already set.', node)
          }

          title = await serializeText(node)
          // Test title
        } else {
          if (lastTest.title.length) {
            fatal('Title already set.', node)
          }

          lastTest.title = await serializeText(node)
        }

        break
      // Code blocks contain input and output for the tests
      case 'code':
        switch (node.lang) {
          case 'http':
            if (lastTest.input.length) {
              fatal('Test input already set.', node)
            }

            lastTest.input = node.value.trim()
            break
          case 'log':
            if (lastTest.output.length) {
              fatal('Test output already set.', node)
            }

            lastTest.output = node.value.trim()
            break
          default:
            fatal(`Unexpected code block with ${node.lang} language.`, node)
            break
        }

        break
      // Test flags, currently unused
      case 'html':
        if (lastTest.flags) {
          fatal('Test flags already set.', node)
        }

        lastTest.flags = JSON.parse(node.value.replace(/^(<!-- meta=)/, '').replace(/(-->)$/, ''))

        break
    }

    if (lastTest.title && lastTest.input && lastTest.output) {
      tests.push(lastTest)
      lastTest = prepareTest()
    }
  }

  // Now generate the Rust code
  let output =
    `
    #[path = "../../src/test_utils.rs"]
    mod test_utils;
    
    use milo_parser_generator::get_span;
    use std::ffi::CString;
    use test_utils::{ create_parser, http, output };
    `.trim() + '\n\n'

  for (let i = 0; i < tests.length; i++) {
    if (i > 0) {
      output += '\n\n'
    }

    const test = tests[i]

    output += `
#[test]
fn ${prefix ? prefix + '_' : ''}${test.title
      .toLowerCase()
      .replaceAll(/[^A-Za-z0-9_]/g, '_')
      .replaceAll('__', '_')}() {
  let parser = create_parser();

  let input = http(r###"
${test.input.trim()}
  "###);

let expected_output = output(r#"
${test.output.trim()}
  "#);

  parser.parse(CString::new(input).unwrap().into_raw());
  assert!(get_span!(debug) == expected_output, "${test.title} parsing failed");
}
`.trim()
  }

  // Format using rustfmt
  const child = spawn('rustup', ['run', 'nightly', '--', 'rustfmt', '--edition', '2021'], {
    stdio: ['pipe', 'inherit', 'inherit']
  })

  let success, fail
  const promise = new Promise((resolve, reject) => {
    success = resolve
    fail = reject
  })

  child.on('close', code => {
    if (code !== 0) {
      fail(new Error(`Process failed with status code ${code}.`))
    }

    success()
  })

  child.stdin.write(output)
  child.stdin.end()

  await promise
}

main()
