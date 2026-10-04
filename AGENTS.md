# Milo development guide

## Parser internals

Milo leverages Rust's [procedural macros](https://doc.rust-lang.org/reference/procedural-macros.html), [syn](https://crates.io/crates/syn), and [quote](https://crates.io/crates/quote) crates to define actions and matchers for the parser.

See the [macros](./macros/README.md) internal crate for more information.

## Build WebAssembly and C++ locally

Required tools:

- [cargo-make](https://github.com/sagiegurari/cargo-make).
- The pinned Rust nightly toolchain, installed via [rustup](https://rustup.rs/).
- [rust-cbindgen](https://github.com/mozilla/cbindgen).
- [Binaryen](https://github.com/WebAssembly/binaryen), providing `wasm-opt`.

Install the pinned toolchain, the Rust sources required by the WebAssembly release build, and the WebAssembly target:

```sh
rustup toolchain install nightly-2026-07-29 --component rust-src
rustup target add wasm32-unknown-unknown
```

Run from the repository root:

```sh
makers
```

This produces debug and release builds for each language in the top-level `dist` folder.

Build tooling is compiled from `scripts` into standalone Rust binaries. Node.js and npm dependencies are not required to build the parser or generate its C++ and WebAssembly packages.

The WebAssembly release build uses immediate-abort panics to keep the artifact smaller. Panics trap without unwinding or rich panic messages. The debug build also enables the `on_state_change` callback and provides more detailed WebAssembly errors.

For JavaScript linting and formatting, install the development dependencies with `pnpm install`.

## Run tests

Run `makers test` from the repository root for the Rust and WebAssembly suites, or `makers test:wasm` to build and test only WebAssembly.

The WebAssembly suite uses Node.js's built-in test runner and tests the release SIMD package by default. Set `MILO_VARIANT=no-simd` to select the non-SIMD package instead. After building, run `pnpm test:wasm` to rerun it without rebuilding.

Tests live in `parser/wasm/test` and mirror the Rust integration tests in `parser/tests` with the same case names: basic, benchmark, compliance, issue, undici, and upgrade. Issue regressions use the `issue_<number>__<description>` naming convention. The llhttp suite is not yet ported.

## Build WebAssembly with Docker

The repository includes a Docker image for building the WebAssembly packages without changing the working tree. Build the image from the repository root, then mount the sources read-only and choose a host directory for the generated artifacts:

```sh
docker build -t milo-wasm .
mkdir -p /path/to/milo-wasm-output
docker run --rm \
  -v "$PWD:/src:ro" \
  -v "/path/to/milo-wasm-output:/output" \
  milo-wasm
```

The container builds both the debug and release profiles in its temporary workspace. The output directory receives the resulting `debug` and `release` packages; the mounted source tree remains read-only.

## Contributing

- Check the latest default branch to make sure the feature hasn't been implemented or the bug hasn't been fixed yet.
- Check the issue tracker for existing requests and contributions.
- The contribution workflow uses a fork and a feature or bugfix branch, followed by commits and a push when the contribution is ready.
- Add tests for changes to prevent regressions.
