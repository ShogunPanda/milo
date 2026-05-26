# Milo Benchmarks

The fixture files in `fixtures/` are human-readable HTTP messages. Real line feeds are ignored, while literal `\r\n` sequences are decoded to CRLF before benchmarking.

Each benchmark case parses roughly 8 GiB of decoded fixture data as a repeated keep-alive stream and prints one Markdown table per fixture.

Benchmarks are comparative smoke tests, not absolute performance claims. Results vary with CPU architecture, frequency scaling, thermal state, operating system, compiler version, Node/V8 version, and background load.

## Fixtures

- `seanmonstar_httparse`: Header-heavy HTTP request with no body.
- `nodejs_http_parser`: HTTP request with headers and a small body.
- `undici`: HTTP response with a large body.

The benchmark loop does not reset parsers between iterations. It measures repeated keep-alive stream parsing.

## C++

The C++ benchmark compares Milo's C++ release build with native llhttp from llhttp's generated GitHub release archive. Both are built with native CPU codegen for benchmark runs.

Build Milo's C++ release artifacts first:

```bash
cd benchmarks/cpp
makers milo
```

The default llhttp source is `nodejs/llhttp` tag `release/v9.3.1`:

```text
LLHTTP_INCLUDE_DIR=../vendor/llhttp-release-v9.3.1/include
LLHTTP_LIB=../vendor/llhttp-release-v9.3.1/build/libllhttp.a
```

Build the default llhttp release archive and run the benchmark:

```bash
cd benchmarks/cpp
makers llhttp
makers verify
```

If llhttp is already available elsewhere, set `LLHTTP_INCLUDE_DIR` and `LLHTTP_LIB`.

Native SIMD is enabled where the compiler and CPU support it. Milo is built with `RUSTFLAGS="-C target-cpu=native"`; llhttp is built with `clang -O3 -DNDEBUG -march=native -flto`.

## WebAssembly

The WebAssembly benchmark compares Milo's release WASM package with Undici's `llhttp_simd.wasm` artifact.

Build Milo's WASM release package, download llhttp's WASM artifact, and run the benchmark:

```bash
cd benchmarks/wasm
makers verify
```

The default llhttp WASM source is pinned to Undici commit `185e6a1513f8f00a9ef9d2cde028e8cce412b11f`:

```text
LLHTTP_WASM=../vendor/llhttp_simd.wasm
LLHTTP_WASM_COMMIT=185e6a1513f8f00a9ef9d2cde028e8cce412b11f
```

Milo WASM is built with `RUSTFLAGS="-C target-feature=+simd128"` and optimized with `wasm-opt -O3 --enable-bulk-memory-opt --enable-simd`. The llhttp artifact is Undici's SIMD build.

## Reporting Results

When publishing benchmark numbers, include:

- CPU architecture and model.
- Operating system.
- Rust, clang, Node, and Binaryen versions.
- Whether the run is local or remote.
- The exact Milo branch or commit.

Prefer wording such as "Milo is competitive with llhttp in these fixtures" or "Milo is faster in this benchmark configuration" instead of broad performance claims.
