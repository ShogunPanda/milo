import generate from '@babel/generator'
import { parse, parseExpression } from '@babel/parser'
import traverse from '@babel/traverse'
import { copyFile, readFile, writeFile } from 'node:fs/promises'
import prettier from 'prettier'
import remarkParse from 'remark-parse'
import remarkStringify from 'remark-stringify'
import { unified } from 'unified'
import prettierConfig from './prettier.config.cjs'

function camelCase(source) {
  return source.toLowerCase().replace(/_(.)/g, (...t) => t[1].toUpperCase().trim())
}

async function generateImports(profile) {
  const imports = {}
  // Open and parse the WASM file
  const mod = await WebAssembly.compile(await readFile(new URL(`../dist/wasm/${profile}/milo.wasm`, import.meta.url)))

  for (const i of WebAssembly.Module.imports(mod)) {
    if (typeof imports[i.module] === 'undefined') {
      imports[i.module] = {}
    }

    let name

    if (i.name.startsWith('__wbg_runCallback')) {
      name = 'runCallback.bind(null, callbackContext)'
    } else if (i.name.startsWith('__wbg_log')) {
      name = 'logger.bind(null, callbackContext)'
    } else {
      name =
        '__' +
        (i.name.startsWith('__wbg_') ? i.name.slice(6).split('_')[0] : camelCase(i.name.replace('__wbindgen_', '')))
    }
    imports[i.module][i.name] = name
  }

  const ast = parse('const imports = ' + JSON.stringify(imports, null, 2), {})

  traverse.default(ast, {
    StringLiteral(path) {
      path.replaceWithSourceString(path.node.value)
    }
  })

  return ast.program.body[0].declarations[0].init.properties
}

async function generateGluecode(profile, version, flags, constants, imports) {
  const getters = []
  const enums = []
  let callbacks

  // Open and parse the JS file
  const template = await readFile(new URL('../src/wasm/template.js', import.meta.url), 'utf-8')
  const ast = parse(template, { sourceType: 'module' })

  // Manipulate the AST
  traverse.default(ast, {
    // Replace placeholder definitions
    FunctionDeclaration(path) {
      if (!path.node.id.name.startsWith('$milo_')) {
        return
      }

      const cleanName = path.node.id.name.slice(6)
      if (cleanName.startsWith('getter_')) {
        const getter = cleanName.slice(7)
        getters.push(getter)

        let converter
        switch (path.node.params[0].name) {
          case 'number':
            converter = '$ >>> 0'
            break
          case 'bigint':
            converter = 'BigInt.asUintN(64, $)'
            break
          case 'bool':
            converter = '$ !== 0'
            break
          default:
            throw new Error(`Unsupported return type ${path.node.params[0].name} for function ${path.node.id.name}`)
        }

        path.insertBefore(
          parseExpression(`
          function ${getter}(parser) {
            return ${converter.replace('$', `this.${getter}(parser)`)}
          }
        `)
        )
      } else if (cleanName === 'callbacks') {
        callbacks = Object.entries(constants)
          .map(c => (c[0].startsWith('CALLBACK_') ? [camelCase('set_' + c[0].replace('CALLBACK_', '')), c[1]] : null))
          .filter(Boolean)

        for (const [callback, index] of callbacks) {
          path.insertBefore(
            parseExpression(`
              function ${callback}(state, parser, cb) {
                state[parser][${index}] = cb
              }
            `)
          )
        }
      } else if (cleanName.startsWith('enum')) {
        const name = cleanName.slice(5)
        const selector = path.node.params[0].name

        const values = Object.entries(constants)
          .map(([k, v]) => (k.startsWith(selector) ? [k.replace(selector, ''), v] : null))
          .filter(Boolean)

        path.insertBefore(
          parse(`
            const ${name} = Object.freeze({
              ${values.map(([k, v]) => `${k}: ${v}`).join(',')},
              ${values.map(([k, v]) => `${v}: '${k}'`).join(',')}
            })
          `).program.body[0]
        )

        enums.push(name)
      }

      path.remove()
    },
    // Replace exports
    Property(path) {
      if (!path.node.key.name?.startsWith('$milo_')) {
        return
      }

      switch (path.node.key.name.slice(6)) {
        case 'wasm':
          // Replace all properties with the corresponding WASM calls
          for (const prop of path.node.value.properties) {
            const newNode = parse(`wasm.${prop.key.name}`)
            path.insertBefore(parseExpression(`{${prop.key.name}: wasm.${prop.key.name}}`).properties[0])
          }

          break
        case 'getters':
          for (const prop of parseExpression(`{ ${getters.map(g => `${g}: ${g}.bind(wasm)`).join(', ')} }`)
            .properties) {
            path.insertBefore(prop)
          }

          break
        case 'callbacks':
          for (const prop of parseExpression(
            `{ ${callbacks.map(c => `${c[0]}: ${c[0]}.bind(wasm, state)`).join(', ')} }`
          ).properties) {
            path.insertBefore(prop)
          }

          break
        case 'enums':
          for (const prop of parseExpression(`{ ${enums.join(', ')} }`).properties) {
            path.insertBefore(prop)
          }

          break
        case 'constants':
          for (const [k, v] of Object.entries(constants)) {
            path.insertBefore(parseExpression(`{ ${k}: ${v} }`).properties)
          }

          break
        case 'version':
          path.insertBefore(parseExpression(`{ version: ${JSON.stringify(version)} }`).properties)
          break
        case 'imports':
          path.insertBefore(imports)
          break
        default:
          throw new Error(`Unsupported placeholder type ${path.node.key.name}`)
      }

      path.remove()
    },
    // Replace Identifiers
    Identifier(path) {
      if (!path.node.name?.startsWith('$milo_')) {
        return
      }

      switch (path.node.name.slice(6)) {
        case 'callback_error_index':
          path.replaceWithSourceString(Object.keys(constants).filter(c => c.startsWith('CALLBACK_')).length)
          break
        case 'flag_debug':
          path.replaceWithSourceString(flags.debug)
          break
      }
    }
  })

  return prettier.format(generate.default(ast).code, { ...prettierConfig, parser: 'babel' })
}

async function generateReadme() {
  const howto = await unified()
    .use(remarkParse)
    .parse('It is usable in JavaScript via [WebAssembly][webassembly].\n\n[webassembly]: https://webassembly.org/')

  // Read the JS file and manipulate as appropriate
  const jsAPI = await unified()
    .use(remarkParse)
    .parse(await readFile(new URL('../../docs/js.md', import.meta.url), 'utf-8'))

  for (const node of jsAPI.children) {
    if (node.type === 'heading') {
      if (node.depth === 1) {
        node.children[0].value = 'API'
      }

      node.depth++
    }
  }

  // Read the README.md file
  const readme = await unified()
    .use(remarkParse)
    .parse(await readFile(new URL('../../README.md', import.meta.url), 'utf-8'))

  let deletingSection = null
  let howtoSectionIndex
  let apiSectionIndex

  // For each node
  for (let i = 0; i < readme.children.length; i++) {
    const node = readme.children[i]

    // When we start a new section, check if we have to delete it
    if (node.type === 'heading' && node.depth === 2) {
      const label = node.children[0].value

      // Finish deleting
      deletingSection = null

      switch (label) {
        case 'How to use it (JavaScript via WebAssembly)':
          howtoSectionIndex = i
          node.children[0].value = 'How to use it'
          break
        case 'How to use it (Rust)':
        // eslint-disable-next-line no-fallthrough
        case 'How to use it (C++)':
        case 'API':
          deletingSection = label

          if (label === 'API') {
            apiSectionIndex = i
          }
          break
      }
    }

    if (deletingSection) {
      readme.children[i] = undefined
    }
  }

  // Add required snippets
  readme.children.splice(apiSectionIndex, 0, ...jsAPI.children)
  readme.children.splice(howtoSectionIndex, 0, howto.children[0])

  // Compact nodes
  readme.children = readme.children.filter(Boolean)

  return unified().use(remarkStringify).stringify(readme)
}

// TODO@PI: TypeScript
async function main() {
  const { version, constants } = JSON.parse(
    await readFile(new URL('../target/buildinfo.json', import.meta.url), 'utf-8')
  )
  const profile = process.argv[2]
  const flags = Object.fromEntries(process.argv[3].split(',').map(p => p.split(':').map(s => s.toLowerCase())))

  // Generate the required files and code
  const imports = await generateImports(profile)
  const glue = await generateGluecode(profile, version, flags, constants, imports)
  const readme = await generateReadme()

  // Copy the package.json by updating the version
  const packageJson = JSON.parse(await readFile(new URL('../src/wasm/package.json', import.meta.url), 'utf-8'))
  packageJson.version = Object.values(version).join('.')
  await writeFile(
    new URL(`../dist/wasm/${profile}/package.json`, import.meta.url),
    JSON.stringify(packageJson, null, 2),
    'utf-8'
  )

  // Create the index file
  await writeFile(new URL(`../dist/wasm/${profile}/index.js`, import.meta.url), glue, 'utf-8')

  // // Rename the WASM file
  // await rename(
  //   new URL(`../dist/wasm/${profile}/milo_bg.wasm`, import.meta.url),
  //   new URL(`../dist/wasm/${profile}/milo.wasm`, import.meta.url)
  // )

  // // Drop the glue code from Rust toolchain
  // await unlink(new URL(`../dist/wasm/${profile}/milo.js`, import.meta.url))

  // Create the README.md file
  await writeFile(new URL(`../dist/wasm/${profile}/README.md`, import.meta.url), readme, 'utf-8')

  // Copy other Markdown files from root
  for (const file of ['CODE_OF_CONDUCT', 'LICENSE']) {
    await copyFile(
      new URL(`../../${file}.md`, import.meta.url),
      new URL(`../dist/wasm/${profile}/${file}.md`, import.meta.url)
    )
  }
}

await main()
