import { spawn } from 'node:child_process'
import { readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'

function info(message) {
  console.log(`\x1b[33m--- [INFO] ${message}\x1b[0m`)
}

function fatal(message) {
  console.error(`\x1b[31m\x1b[1m--- [ERROR] ${message}\x1b[0m`)
  process.exit(1)
}

function execute(title, command, ...args) {
  const verbose = process.env.VERBOSE === 'true'

  info(`${title} (${command} ${args.join(' ')}) ...`)
  const child = spawn(command, args)

  let stdout = ''
  let stderr = ''
  return new Promise((resolve, reject) => {
    child.stdout.on('data', chunk => {
      stdout += chunk.toString('utf-8')

      if (verbose) {
        process.stdout.write(chunk.toString('utf-8'))
      }
    })

    child.stderr.on('data', chunk => {
      stderr += chunk.toString('utf-8')

      if (verbose) {
        process.stderr.write(chunk.toString('utf-8'))
      }
    })

    child.on('close', code => {
      if (verbose) {
        process.stderr.write('\n')
      }

      if (code !== 0) {
        const error = new Error(`Command failed with code ${code}. Aborting ...`)

        reject(error)
        return
      }

      resolve({ stdout, stderr })
    })
  })
}

async function main() {
  // Check if the tree is not clean and eventually abort
  const status = await execute('Verifying GIT status', 'git', 'status', '-s')

  if (status.stdout.toString('utf-8').trim().length > 0 && process.env.ALLOW_DIRTY_TREE !== 'true') {
    fatal('Cannot publish if the working directory is not clean.')
  }

  // Get the latest version
  const latestVersionCheck = await execute('Getting latest published version', 'git', 'tag')
  let latestVersion = latestVersionCheck.stdout.toString('utf-8').split('\n').sort().at(-1).slice(1)
  if (latestVersion.length === 0) {
    latestVersion = '0.0.0'
  }

  // Get the new version
  const newVersion = JSON.parse(
    await readFile(new URL('../dist/wasm/release/@perseveranza-pets/milo/package.json', import.meta.url), 'utf-8')
  ).version
  info(`Publishing version ${newVersion} (from ${latestVersion})`)

  // Publish on Cargo

  // Publish on NPM
  process.chdir(fileURLToPath(new URL('../dist/wasm/release/@perseveranza-pets/milo/', import.meta.url)))
  await execute('Publishing on NPM', 'npm', 'publish', '--access', 'public1')

  // Save tags
  process.chdir(fileURLToPath(new URL('..', import.meta.url)))
  await execute('Saving GIT tag', 'git', 'tag', `v${newVersion}`)
  await execute('Saving GIT tag', 'git', 'push', 'origin', '--tags')

  // Generate release on GitHub
  await execute('Creating release on GitHub', 'gh', 'release', 'create', '-d', '--generate-notes', `v${newVersion}`)
}

try {
  await main()
} catch (e) {
  fatal(e.message)
}
