import { readFile } from 'node:fs/promises'
import semver from 'semver'
import YAML from 'yaml'

async function readYamlList (name) {
  const raw = await readFile(new URL(`../macros/constants/${name}.yml`, import.meta.url), 'utf-8')
  return YAML.parse(raw)
}

async function readVersion () {
  const raw = await readFile(new URL('../parser/Cargo.toml', import.meta.url), 'utf-8')
  const version = raw.match(/^\s*version\s*=\s*"([^"]+)"/m)?.[1]

  if (!version) {
    throw new Error('Cannot find parser version in parser/Cargo.toml')
  }

  const parsed = semver.parse(version)

  if (!parsed) {
    throw new Error(`Invalid parser version in parser/Cargo.toml: ${version}`)
  }

  return {
    raw: version,
    major: parsed.major,
    minor: parsed.minor,
    patch: parsed.patch,
    prerelease: parsed.prerelease.join('.')
  }
}

export async function getBuildInfo () {
  const [version, methods, errors, callbacks, states] = await Promise.all([
    readVersion(),
    readYamlList('methods'),
    readYamlList('errors'),
    readYamlList('callbacks'),
    readYamlList('states')
  ])
  const constants = {}

  for (const [i, method] of methods.entries()) {
    constants[`METHOD_${method.replaceAll('-', '_')}`] = i
  }

  for (const [i, callback] of callbacks.entries()) {
    constants[`CALLBACK_${callback.toUpperCase()}`] = i
  }

  let all = 0
  constants.CALLBACK_ACTIVE_NONE = 0
  for (const [i, callback] of callbacks.entries()) {
    const bit = 1 << i
    constants[`CALLBACK_ACTIVE_${callback.toUpperCase()}`] = bit
    all |= bit
  }
  constants.CALLBACK_ACTIVE_ALL = all

  for (const [i, error] of errors.entries()) {
    constants[`ERROR_${error}`] = i
  }

  for (const [i, state] of states.entries()) {
    constants[`STATE_${state.toUpperCase()}`] = i
  }

  return { version, constants }
}

if (import.meta.main) {
  console.log(JSON.stringify(await getBuildInfo(), null, 2))
}
