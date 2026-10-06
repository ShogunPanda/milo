# Milo Fuzzing Pipeline for Security Vulnerabilities

**Goal.** Stand up a repeatable, coverage-guided fuzzing pipeline that finds
memory-safety bugs, parser state-machine bugs, and — in particular —
**HTTP request smuggling** primitives in Milo's HTTP/1.1 parser across its
native Rust, C ABI, and WebAssembly surfaces.

**Why a plan.** Milo is a hand-written HTTP/1.1 parser with a large amount of
`unsafe` raw-pointer code (`parser/src/parse.rs`), a fixed 64 KiB event buffer,
and retained-input buffer management (`unconsumed`/`manage_unconsumed`). It
compiles to three artifacts (Rust, C++ via cbindgen, WebAssembly), so bugs can
surface differently per target. Request smuggling is the highest-value bug class
for an HTTP parser: it arises from *framing disagreements* (Content-Length vs.
Transfer-Encoding, duplicate/ambiguous headers, obs-fold, whitespace handling)
and is best found with **oracle/differential** fuzzing, not just crash finding.

**Status (this plan).** The cargo-fuzz scaffold is in place and builds cleanly:

- `parser/fuzz/Cargo.toml` — fuzz crate (not a workspace member, so the normal
  build/test flow is unaffected).
- `parser/fuzz/src/lib.rs` — shared `Framing` / `MessageFraming` helpers and the
  smuggling predicate.
- `parser/fuzz/fuzz_targets/{bytes,http,smuggling}.rs` — the three targets;
  `cargo check` passes for all of them.
- `parser/fuzz/tests/conformance.rs` — `#[test]` regression battery (runs under
  Miri) that pins the smuggling invariants; `cargo test` passes.
- `parser/fuzz/corpus/seeds/` — hand-seeded smuggling inputs.
- `parser/fuzz/.gitignore` and `[tasks.fuzz]` entries in `parser/Makefile.toml`.
- `parser/tests/smuggling.rs` — pure-`milo_parser` smuggling regression tests in
  the main suite (also runs under Miri).
- `.github/workflows/fuzz.yml` — nightly + opt-in fuzz workflow; `ci.yml` gained
  a fast `fuzz-build` gate. (See §9.)
- `cargo-fuzz` was installed and the three targets were each run for 20k
  iterations locally with no crashes/leaks.

Still to do: refresh the corpus from the llhttp fixtures, run sustained nightly
fuzzing, wire the (optional) differential reference parser, and add the WASM/C++
harnesses.

---

## 1. Attack Surface & Threat Model

### In scope
- `Parser::parse(&mut self, input: *const c_uchar, limit: usize) -> usize` — the
  core byte-level entry point (Rust).
- C ABI surface in `parser/src/native.rs`: `milo_create`, `milo_parse`,
  `milo_destroy`, `milo_reset`, `milo_finish`, `milo_complete`,
  `milo_set_active_events`, `milo_set_max_body_payload`,
  `milo_set_suspend_after_headers`.
- Header/body framing state machine: request & response lines, header fields,
  `Content-Length`, `Transfer-Encoding` (incl. `chunked` and multiple encodings),
  chunk extensions, trailers, obs-fold, OWS stripping, upgrade/tunnel, keep-alive.
- Config toggles that alter control flow: `autodetect`, `is_request`,
  `manage_unconsumed`, `skip_body`, `suspend_after_headers`,
  `continue_without_data`, `max_body_payload`, `debug`.

### Out of scope
- HTTP/2, TLS, application-level consumers of emitted events, and the
  `references/*` sample executables themselves.

### Bug classes we are hunting
1. **Memory safety** in the `unsafe` parser: out-of-bounds read/write on the
   input slice or the fixed 64 KiB `events` buffer, use-after-free / double-free
   of the retained `unconsumed` buffer, and UB from `from_raw_parts`/`Box::from_raw`.
2. **Request smuggling** (CL.TE, TE.CL, duplicate `Content-Length`,
   `Transfer-Encoding` not ending in `chunked`, folded `Content-Length`,
   whitespace/obs-fold ambiguity, header-name/value validation gaps).
3. **State-machine confusion**: parser hangs, infinite loops, or accepts a
   message it should reject (or vice versa) relative to a reference parser.
4. **Resource exhaustion**: unbounded retention/aggregation of `unconsumed`,
   and inconsistent handling when `suspend_after_headers`/`max_body_payload`
   interplay with framing.

---

## 2. Fuzzing Strategy (Layered)

We combine four complementary layers. Crashes are the floor; **smuggling and
divergence** are the ceilings.

### Layer 1 — Coverage-guided byte-level fuzzing (`bytes`)
Feed arbitrary bytes directly to `parse` with every config toggle flipped.
Finds crashes/UB in the quickest possible time and explores the state machine
periphery (partial lines, stray CR, NUL bytes, huge declared lengths).

### Layer 2 — Structure-aware generation (`http`)
Use the `arbitrary` crate to generate *valid-ish* HTTP messages (method, URL,
headers, body, chunked framing with extensions, trailers), then serialize to
wire bytes. Reaches deep states (chunked bodies, trailers, obs-fold, upgraded
connections) that random bytes rarely construct, and avoids wasting cycles on
95% garbage prefixes. Must be combined with a byte-mutating pass so the fuzzer
can *break* the structure (that's where smuggling lives).

### Layer 3 — Request-smuggling oracle (`smuggling`) [primary smuggling target]
Two predicates, both run on every input:
- **RFC-conformance oracle (self-contained, always on):** parse the input; if it
  is *accepted* (no error) but violates a framing invariant that RFC 9112 makes a
  MUST (CL+TE present together, TE not ending in `chunked`, duplicate/conflicting
  `Content-Length`), then we have a class of "parser accepted a smuggling
  primitive" finding → minimize and report.
- **Differential oracle (external reference, feature-gated):** run the *same*
  bytes through Milo and through a reference HTTP parser, then compare the chosen
  framing and the end-of-message boundary. **Any disagreement is a
  request-smuggling bug** when Milo is deployed as one side of a proxy pair.
  The natural reference is **llhttp** (Milo already imports and passes the llhttp
  suite — see `parser/tests/fixtures/llhttp/` and `references/`). Because the
  reference requires an external dependency, this sub-target is compiled under a
  cargo feature (`oracle`) so the default build stays dependency-free.

### Layer 4 — Memory-safety tooling
- **ASan/LSan**: `cargo-fuzz` builds with `-Z sanitizer=address` by default on
  nightly, which also catches the retained-buffer leaks `parser/tests/issues.rs`
  tracks manually.
- **Miri**: run the existing and new conformance tests under `cargo +nightly miri`
  to catch UB that ASan can miss (e.g., `from_raw_parts` with bad provenance,
  transmute/alignment, `Box::from_raw` misuse).
- **Valgrind** (C++ build) and **Wasm fuzzing** `@jazzer.js` (WASM traps on OOB).

---

## 3. Repository Layout to Add

```
parser/fuzz/                    # cargo-fuzz crate (not part of normal build)
  Cargo.toml
  fuzz_targets/
    bytes.rs                    # Layer 1: raw byte fuzzing + config toggles
    http.rs                     # Layer 2: structured generation
    smuggling.rs                # Layer 3: conformance + (optional) llhttp differential
  corpus/
    requests/                   # seeds copied from parser/tests/fixtures/llhttp/requests
    responses/                  # seeds from parser/tests/fixtures/llhttp/responses
    smuggling/                  # hand-written CL.TE / TE.CL / dup-CL seeds
README.md                       # how to build & triage
```

Helpers live in crate-private modules (e.g., `parser/fuzz/src/`) so targets can
share framing logic. We also add a small `#[cfg(test)]` conformance suite that
runs under **Miri** and doubles as the regression oracle once a finding is fixed.

---

## 4. Harness Design

### 4.1 `parser/fuzz/Cargo.toml`

```toml
[package]
  name    = "milo-fuzz"
  version = "0.0.0"
  edition = "2024"
  publish = false

[dependencies]
  arbitrary     = { version = "1", features = ["derive"] }
  libfuzzer-sys = "0.4"
  milo-parser   = { path = ".." }

[features]
  # Gates the differential oracle module. It is a no-op until a reference parser
  # crate is wired in (see the `oracle` module in smuggling.rs). Kept optional so
  # the default fuzz build stays dependency-free.
  oracle = []

[[bin]]
  name = "bytes"
  path = "fuzz_targets/bytes.rs"
  test = false
  doc = false
  bench = false

[[bin]]
  name = "http"
  path = "fuzz_targets/http.rs"
  test = false
  doc = false
  bench = false

[[bin]]
  name = "smuggling"
  path = "fuzz_targets/smuggling.rs"
  test = false
  doc = false
  bench = false
```

> `parser/fuzz/target/` is already covered by the repo's `**/target` gitignore.
> Commit `parser/fuzz/Cargo.lock` for reproducible fuzzing.

### 4.2 `parser/fuzz/src/lib.rs` — shared framing helpers

```rust
//! Shared framing classification + conformance checks used by the targets.

use milo_parser::Parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framing {
  Closure,                 // no CL, no TE -> body runs until connection close
  ContentLength(u64),      // body bounded by Content-Length
  Chunked(u64),            // Transfer-Encoding (last coding chunked); u64 = CL if both present
  UnchunkedTransfer,       // TE present but last coding is NOT chunked
}

/// Derive the body framing a parser chose for the *current* message.
/// Call from an `on_headers` / `on_message_complete` snapshot, or read the
/// parser's public flags right after a successful parse.
pub fn framing(p: &Parser) -> Framing {
  if p.has_transfer_encoding {
    if p.has_chunked_transfer_encoding {
      Framing::Chunked(p.content_length)
    } else {
      Framing::UnchunkedTransfer
    }
  } else if p.has_content_length {
    Framing::ContentLength(p.content_length)
  } else {
    Framing::Closure
  }
}

/// RFC 9112 §6.3 smuggling primitives that Milo MUST reject.
/// Returns `None` if the message is conforming, or a description of the
/// violation. Call only when `p.error_code == ERROR_NONE` (i.e. accepted).
pub fn conformance_violation(p: &Parser) -> Option<&'static str> {
  if p.has_transfer_encoding && p.has_content_length {
    // CL.TE / TE.CL: the classic smuggling vector. MUST NOT send both.
    Some("both Transfer-Encoding and Content-Length present")
  } else if p.has_transfer_encoding && !p.has_chunked_transfer_encoding {
    Some("Transfer-Encoding present but last coding is not chunked")
  } else {
    None
  }
}
```

> Duplicate/conflicting `Content-Length` and obs-fold detection require capturing
> header *values* via `on_header_value` callbacks; see the smuggler detail below.

### 4.3 `parser/fuzz/fuzz_targets/bytes.rs` — Layer 1

```rust
#![no_main]
#[macro_use]
extern crate libfuzzer_sys;

use milo_parser::{CALLBACK_ACTIVE_ALL, Parser};

// The parser's event buffer is a fixed 64 KiB. Feeding more than that just
// exercises the suspend/retain paths and wastes cycles.
const MAX_FEED: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
  let limit = data.len().min(MAX_FEED);

  // (a) autodetect mode, all events on.
  let mut p = Parser::new();
  p.active_events = CALLBACK_ACTIVE_ALL;
  let _ = p.parse(data.as_ptr(), limit);

  // (b) forced request / response, with unconsumed management so the retained
  // buffer and cross-call aggregation paths are exercised.
  for is_request in [false, true] {
    let mut p = Parser::new();
    p.autodetect = false;
    p.is_request = is_request;
    p.active_events = CALLBACK_ACTIVE_ALL;
    p.manage_unconsumed = true;
    if data.len() > 1 {
      let (a, b) = data.split_at(data.len() / 2);
      let _ = p.parse(a.as_ptr(), a.len());
      let _ = p.parse(b.as_ptr(), b.len()); // prepends retained bytes
    } else {
      let _ = p.parse(data.as_ptr(), limit);
    }
  }

  // (c) body-skipping mode (surfaces skip_body / suspend_after_headers paths).
  let mut p = Parser::new();
  p.active_events = CALLBACK_ACTIVE_ALL;
  p.skip_body = true;
  p.suspend_after_headers = true;
  let _ = p.parse(data.as_ptr(), limit);
});
```

### 4.4 `parser/fuzz/fuzz_targets/http.rs` — Layer 2 (structure-aware)

```rust
#![no_main]
#[macro_use]
extern crate libfuzzer_sys;

use arbitrary::{Arbitrary, Unstructured};
use milo_parser::{CALLBACK_ACTIVE_ALL, Parser};

#[derive(Arbitrary, Debug)]
struct FuzzMessage {
  is_request: bool,
  method: Vec<u8>,      // arbitrary token/space bytes -> often makes nonsense, good
  url: Vec<u8>,
  version: Vec<u8>,
  headers: Vec<FuzzHeader>,
  body: Vec<u8>,        // used for Content-Length-style bodies
  chunked: bool,
  trailer_headers: Vec<FuzzHeader>,
}

#[derive(Arbitrary, Debug)]
struct FuzzHeader { name: Vec<u8>, value: Vec<u8> }

impl FuzzMessage {
  fn to_wire(&self) -> Vec<u8> {
    let mut out = Vec::new();
    if self.is_request {
      out.extend_from_slice(&self.method);
      out.extend_from_slice(b" ");
      out.extend_from_slice(&self.url);
      out.extend_from_slice(b" ");
      out.extend_from_slice(&self.version);
      out.extend_from_slice(b"\r\n");
    } else {
      out.extend_from_slice(b"HTTP/1.1 200 OK\r\n");
    }
    for h in &self.headers {
      out.extend_from_slice(&h.name);
      out.extend_from_slice(b": ");
      out.extend_from_slice(&h.value);
      out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    if self.chunked {
      let mut left = self.body.len();
      let mut i = 0;
      while left > 0 {
        let chunk = (left % 17 + 1).min(left);
        out.extend_from_slice(format!("{:x}\r\n", chunk).as_bytes());
        out.extend_from_slice(&self.body[i..i + chunk]);
        out.extend_from_slice(b"\r\n");
        i += chunk;
        left -= chunk;
      }
      out.extend_from_slice(b"0\r\n");
      for t in &self.trailer_headers {
        out.extend_from_slice(&t.name);
        out.extend_from_slice(b": ");
        out.extend_from_slice(&t.value);
        out.extend_from_slice(b"\r\n");
      }
      out.extend_from_slice(b"\r\n");
    } else {
      out.extend_from_slice(&self.body);
    }
    out
  }
}

fn run(bytes: &[u8]) {
  let limit = bytes.len().min(64 * 1024);
  let mut p = Parser::new();
  p.active_events = CALLBACK_ACTIVE_ALL;
  let _ = p.parse(bytes.as_ptr(), limit);
}

fuzz_target!(|data: &[u8]| {
  // Structured generation...
  if let Ok(msg) = FuzzMessage::arbitrary(&mut Unstructured::new(data)) {
    run(&msg.to_wire());
  }
  // ...plus a raw byte pass so structure can be *mutated* into smuggling.
  run(data);
});
```

### 4.5 `parser/fuzz/fuzz_targets/smuggling.rs` — Layer 3 (primary)

The committed harness (see the file in the repo) installs `on_message_start`,
`on_headers`, `on_header_name`, `on_header_value` and `on_message_complete`
callbacks to snapshot, per message, the framing Milo chose and every
`Content-Length` value it saw. It runs the input in autodetect, forced-request
and forced-response modes, then panics if Milo *accepted* a message that is a
smuggling primitive:

```text
if m.has_transfer_encoding && m.has_content_length              -> CL.TE / TE.CL
if m.has_transfer_encoding && !m.has_chunked_transfer_encoding  -> TE not ending in chunked
if m.content_lengths.len() > 1 && differing values              -> duplicate/conflicting CL
```

The panic makes `cargo-fuzz` record and minimize the input as a finding. The
derived framing (`content_lengths`, `has_*` flags) can later be compared against a
reference parser in the `#[cfg(feature = "oracle")]` module, which is currently
a documented stub because adding the reference crate (e.g. the llhttp bindings)
requires pinning its upstream version. The self-contained conformance tripwire
above needs no external dependency, so the default build and CI run it.

> **Important:** panicking in a fuzz target is what makes `cargo-fuzz` record a
> finding and then minimize it. For conformance findings we want minimization to
> keep the *interesting* (small) prefix, so a dedicated "report" harness that
> stores the violating input is preferable to a bare panic; see §7.

---

## 5. Request-Smuggling Oracle — Logic Detail

Smuggling is fundamentally a *two-parser* property. We model the two ways it
shows up.

### 5.1 Conformance oracle (self-contained, always on)
A front end and a back end disagree most often because one of them accepts a
message the RFC says it MUST reject. If Milo accepts any of these, it can be the
*lenient* half of a smuggling pair:

- `Transfer-Encoding` **and** `Content-Length` in the same message (CL.TE / TE.CL).
- `Transfer-Encoding` whose last coding is **not** `chunked`.
- Duplicate `Content-Length` with conflicting values (or a `Content-Length`
  header that is folded via obs-fold so one parser sees it and the other doesn't).
- `Content-Length: 0` + `Transfer-Encoding: chunked`-style ambiguity where
  whitespace/`\r`/`\n` handling differs.

To capture *duplicate* and *folded* `Content-Length`, the oracle registers
`on_header_name` / `on_header_value` callbacks and records every `content-length`
field; it then flags any stream with two different numeric values, or any value
preceded by an obs-fold (leading SP/HT) continuation.

### 5.2 Differential oracle (external reference, feature-gated)
For the same byte stream, Milo and the reference must agree on **where message 1
ends** and **how the body is framed**. We compare:

- end-of-message offset (`consumed` after a single `parse`, plus
  `on_message_complete` positions),
- framing class (`Closure` / `ContentLength(n)` / `Chunked`),
- accept vs. reject.

A difference means the two sides will split the byte stream differently →
smuggling/desync. Because Milo already imports and passes the llhttp suite, llhttp
is the correct default oracle; we gate it behind the `oracle` feature to keep the
default build dependency-free. An alternative that avoids a new C dependency is to
drive the **reference executables** in `references/rust` (already built by
`makers references`) as an out-of-process oracle, but that is too slow for
in-process fuzzing and should only be used for corpus generation / spot checks.

---

## 6. Memory-Safety Tooling

| Tool | What it catches | How to run |
|------|-----------------|------------|
| ASan via `cargo-fuzz` | OOB reads/writes, use-after-free, leaks (LSan) | `cargo fuzz run <target> -runs=0 -max_len=65536` |
| **Miri** | Undefined behavior ASan misses (provenance, alignment, raw-pointer misuse) | `cargo +nightly miri test --test <conformance>` (+ copy the retained-buffer tracking idea from `tests/issues.rs`) |
| Valgrind | C++ build memory errors | build C++ and run `valgrind` on the fuzz driver |
| Jazzer.js / wasm-fuzz | WASM OOB (traps) | feed a JS harness to `@jazzer.js` against `dist/wasm/...` |

Add a **Miri-targeted conformance test** in `parser/fuzz/src/lib.rs` (or a new
`parser/tests/fuzz_conformance.rs`) that runs a battery of requests/responses and
asserts no panic; this is reused to pin down regressions and is the fastest way to
catch the raw-pointer and `Box::from_raw` bugs. Note that `milo_parser` is built
with `panic='abort'` only for release/WASM; a normal `cargo test`/Miri build uses
unwind, so it works.

---

## 7. Corpus & Seed Inputs

Harvest seeds from existing, already-curated inputs — this gives immediate deep
coverage instead of starting cold:

- `parser/tests/fixtures/llhttp/requests/*.yml` (164 files) → `corpus/requests/`
- `parser/tests/fixtures/llhttp/responses/*.yml` (82 files) → `corpus/responses/`
- `parser/tests/basic.rs`, `compliance.rs`, `issues.rs`, `undici.rs`,
  `upgrade.rs` → extract raw wire bytes → `corpus/`.
- Hand-written `corpus/smuggling/` seeds, e.g.:
  - `POST / HTTP/1.1\r\nContent-Length: 6\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\nGET / HTTP/1.1\r\n\r\n`
  - `POST / HTTP/1.1\r\nContent-Length: 6\r\nTransfer-Encoding: chunked\r\n\r\n6\r\n0\r\n\r\n`
  - `POST / HTTP/1.1\r\nContent-Length: 4\r\nContent-Length: 6\r\n\r\n`
  - `POST / HTTP/1.1\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n`
  - obs-fold: `GET / HTTP/1.1\r\nContent-Length: 6\r\n Content-Length: 4\r\n\r\n`

Use the existing `scripts` import helper (`makers llhttp:import`) to keep the
llhttp corpus in sync, and a small `scripts`-style dump to convert each YAML
`input` list into raw bytes for the corpus.

---

## 8. Build & Run Commands

```sh
# 1) Install the fuzzer. The active toolchain is already nightly-2026-07-29.
cargo install cargo-fuzz

# 2) Seed the corpus (one-time / refresh).
#    ... copy from parser/tests/fixtures/llhttp/{requests,responses} ...

# 3) Run each layer. Default ASan; use -max_total_time for bounded CI runs.
cd parser/fuzz
cargo fuzz run bytes     -- -max_total_time=300 -max_len=65536
cargo fuzz run http      -- -max_total_time=300 -max_len=65536
cargo fuzz run smuggling -- -max_total_time=300 -max_len=65536

# 4) Conformance regression battery (also runs under Miri).
cargo test --test conformance

# 5) (Optional) differential oracle against a reference parser. The `oracle`
#    feature gates the module; wire the reference crate (e.g. llhttp bindings)
#    and implement `oracle::compare` first, then run:
cargo fuzz run smuggling --features oracle -- -max_total_time=300

# 6) Corpus growth / coverage telemetry.
cargo fuzz fmt bytes
cargo fuzz coverage smuggling

# 7) Minimize a finding (from parser/fuzz).
cargo fuzz tmin smuggling -- -exact_artifact_path=... # or -artifact_prefix
```

CI hooks go into a new `[tasks.fuzz]` block in `parser/Makefile.toml` (and so the
top-level `makers` can dispatch to it), gated so it does not run on every PR
(see §9).

---

## 9. CI Integration

Two pieces are wired up:

**`fuzz.yml`** (dedicated nightly/on-demand workflow, not every PR):
- Triggers: `schedule` (nightly `0 3 * * *`), `workflow_dispatch`, `push` to main
  (short smoke), and `pull_request` **only when the `fuzz` label is applied**.
- Jobs:
  - `conformance` — fast smuggling regression battery (`cargo test --test
    conformance` in the fuzz crate, plus `--test smuggling` in the parser crate).
  - `fuzz` (matrix `bytes`/`http`/`smuggling`) — installs `cargo-fuzz`, seeds the
    corpus from `corpus/seeds/`, then runs `cargo fuzz run <target>` with a
    bounded `-max_total_time` (900 s nightly/dispatch, 120 s on push/PR) and
    `-artifact_prefix`. On failure it uploads the crash artifacts; it always
    uploads a corpus snapshot for inspection.
  - `miri` — `cargo +nightly miri test --test smuggling --test issues` to catch UB
    in the parser's `unsafe` code that ASan cannot.

**`ci.yml`** (fast gate on every PR/push):
- A new `fuzz-build` job runs `cargo check --all-targets` in `parser/fuzz` and
  `cargo test --test conformance`, so the fuzz harnesses must stay compile-clean.
- `makers test` in the existing `ci` job already runs `cargo test` in `parser/`,
  which now also includes `tests/smuggling.rs`.

```
parser/fuzz/artifacts/        # crash outputs, uploaded on failure
parser/fuzz/corpus/           # growing corpus (commit only curated seeds)
```

```
parser/fuzz/artifacts/        # cargo-fuzz default crash output dir
parser/fuzz/corpus/           # growing corpus (commit only curated seeds)
```

---

## 10. Triage, Minimization & Regressions

1. **Collect**: artifacts land in `parser/fuzz/artifacts/` (each a small binary
   file). A finding is either (a) a crash/leak, or (b) a "report" produced by the
   smuggling oracle (framing divergence / accepted CL.TE).
2. **Minimize**: `cargo fuzz tmin <target> -- <artifact>` -> smallest repro.
3. **Reproduce as a Rust test**: add a `#[test]` to `parser/tests/` using the
   harness helpers (`tests/helpers/{mod,llhttp}.rs`) — mirror the existing
   `issue_<n>__<desc>` convention and the `issues.rs` memory-tracking harness.
4. **Fix & pin**: write the regression as a conformance test that (a) asserts the
   buggy input is now handled correctly, and (b) runs under Miri.
5. **Re-fuzz** the affected target to confirm no regression and new coverage.

---

## 11. Metrics & Success Criteria

- **Zero crashes/leaks** across all three targets after a sustained run (e.g.,
  24 h of multi-core fuzzing on the Rust target; bounded CI runs).
- The `smuggling` oracle produces **no accepted CL+TE / non-chunked-TE** inputs,
  and (with `oracle` feature) **no divergence** vs. llhttp on the seed corpus and
  after fuzzing.
- **Code coverage** of `parse.rs` grows monotonically during early runs and then
  plateaus (use `cargo fuzz coverage`); pay attention to the body/chunk/trailer
  and error branches.
- A **corpus growth** trend that flattens indicates state-machine saturation.
- Each confirmed bug gets a regression test and a `PLAN.md`/CHANGELOG note.

---

## 12. Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Running a fixed 64 KiB event buffer with larger inputs wastes cycles | Cap `parse` feed to **64 KiB** (`MAX_FEED`) in all harnesses; fuzz body sizes within that budget. |
| Fuzzer hangs on pathological inputs (state-machine livelock) | Use `-timeout` (default 1200 s) and add a `-rss_limit_mb`; the parser uses `suspend!()`/`paused`, so streams that don't terminate will be caught by the deadline. |
| `cargo-fuzz` needs nightly + sanitizer; install friction | The repo already pins `nightly-2026-07-29`; document `cargo install cargo-fuzz` once. Keep the `oracle` feature optional so llhttp's C toolchain is not a blocker. |
| WAsm target not installed in CI | `rustup target add wasm32-unknown-unknown` (already in the main CI), and install binaryen for `wasm-opt`. |
| Differential oracle adds a heavy C dependency | Feature-gate it; fall back to the self-contained conformance oracle for the default/CI run, and use `references/rust` for one-off cross-checks. |
| False positives from the conformance oracle (Milo may intentionally be lenient) | Scope the oracle to **RFC MUSTs** (CL+TE, TE-not-chunked, duplicate CL). For any lenient-by-design case, whitelist with a comment and only assert the *ambiguity* class. |
| Miri is slow | Run it only on the curated conformance battery, not during coverage fuzzing. |
| Corpus bloat | Commit only a small curated corpus; let CI re-grow it and upload as artifacts. |

---

## 13. Milestones / Phases

- **Phase 0 — Scoping & deps.** Add `parser/fuzz/Cargo.toml`, `cargo install
  cargo-fuzz`, confirm nightly. Add `[tasks.fuzz]` to `Makefile.toml`.
- **Phase 1 — Crash finding.** Land `bytes.rs` + `http.rs`; seed corpus from the
  llhttp fixtures; run with ASan; triage and fix crashes; add regression tests.
- **Phase 2 — Smuggling oracle.** Land `smuggling.rs` (self-contained conformance
  oracle) + `shared` framing helpers; build a `corpus/smuggling/` seed set;
  confirm no accepted CL.TE/TE.CL.
- **Phase 3 — Differential oracle (optional).** Wire the llhttp-backed reference
  behind the `oracle` feature; compare framing/end-of-message on the corpus.
- **Phase 4 — Multi-target & memory.** Add C ABI fuzz driver (Valgrind), WASM
  fuzzing (Jazzer.js), and a Miri conformance test.
- **Phase 5 — CI & hardening.** Nightly workflow with artifact upload, crash
  triage runbooks, and a CHANGELOG/`PLAN.md` record of all findings.

---

## 14. Immediate Next Steps

1. `cargo install cargo-fuzz` and confirm `cargo fuzz --version`.
2. Add `parser/fuzz/Cargo.toml` + the three `fuzz_targets/*.rs` and the shared
   `src/lib.rs` as scaffolded above.
3. Seed corpus from `parser/tests/fixtures/llhttp/{requests,responses}`.
4. Run `cargo fuzz run bytes/http` for a short bounded window and triage.
5. Land the `smuggling` conformance oracle; run it over the seed corpus to
   establish a baseline (it should find zero accepted CL.TE primitives).
6. Add the Miri conformance test and wire the nightly CI workflow.
