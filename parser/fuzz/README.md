# Milo Fuzzing

Coverage-guided fuzzing for the `milo-parser` HTTP/1.1 parser. The primary goal
is finding memory-safety and parser bug classes, with a dedicated oracle for
**HTTP request smuggling** primitives.

See [`PLAN.md`](../../PLAN.md) at the repository root for the full rationale,
targets, and CI/triage strategy.

## Layout

| File                          | Purpose |
|-------------------------------|---------|
| `fuzz_targets/bytes.rs`       | Raw byte-level fuzzing with every config toggle (crash/UB finder). |
| `fuzz_targets/http.rs`        | Structure-aware generation (chunked bodies, trailers, CL bodies). |
| `fuzz_targets/smuggling.rs`   | Request-smuggling conformance oracle (CL+TE, TE-not-chunked, dup CL). |
| `src/lib.rs`                  | Shared framing helpers and the smuggling predicate. |

## Setup

This crate is **not** part of the normal Milo build (it is not a workspace
member). `cargo-fuzz` builds it only when you invoke `cargo fuzz`.

```sh
cargo install cargo-fuzz
```

The active toolchain is already `nightly-2026-07-29`, which supports the
sanitizer flags `cargo-fuzz` uses.

## Run

```sh
cd parser/fuzz

# Seed the corpus (see below), then run each target with ASan.
cargo fuzz run bytes     -- -max_total_time=300 -max_len=65536
cargo fuzz run http      -- -max_total_time=300 -max_len=65536
cargo fuzz run smuggling -- -max_total_time=300 -max_len=65536

# Differential middleware oracle (requires the optional llhttp dependency).
cargo fuzz run smuggling --features oracle -- -max_total_time=300

# Minimize a recorded finding.
cargo fuzz tmin smuggling -- -artifact_prefix=...
```

## Seed corpus

Reuse the already-curated llhttp fixtures for instant deep coverage:

```sh
mkdir -p corpus/requests corpus/responses corpus/smuggling
cp ../tests/fixtures/llhttp/requests/*.corpus corpus/requests/ # see note below
```

The `*.yml` files in `parser/tests/fixtures/llhttp/{requests,responses}` are
structured; convert each `input` list to raw wire bytes before copying into the
corpus (a tiny exporter in `scripts/` can do this, or use `raw` inputs verbatim).
Hand-seed `corpus/smuggling/` with CL.TE / TE.CL / duplicate-`Content-Length`
variants.

## Triage

Crashing inputs land in `parser/fuzz/artifacts/`. For each one:

1. Minimize: `cargo fuzz tmin <target> -- <artifact>`.
2. Reproduce as a Rust test under `parser/tests/` (mirror the `issue_<n>__<desc>`
   convention) using the helpers in `parser/tests/helpers/`.
3. Fix, then add the regression as a conformance test that also runs under Miri:
   `cargo +nightly miri test`.
