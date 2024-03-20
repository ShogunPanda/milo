// TODO@PI: Avoid shapes
const callbacksRegistry = {}

function runCallback(parser, type, at, len) {
  const value = callbacksRegistry[parser][type]?.(at, len) ?? 0

  if (typeof value !== 'number') {
    throw new TypeError(`Callback for ${module.exports.Callbacks[type]} must return a number, got ${typeof value}.`)
  }

  return 0
}
