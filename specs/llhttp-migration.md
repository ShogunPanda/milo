# llhttp Markdown -> YAML Importer (Current Behavior)

This document reflects the importer behavior implemented in this repository as of this session.

## 1) Objective

Maintain the Node.js ESM importer:

- `parser/tools/import-llhttp-tests.js`

It converts llhttp markdown tests into Milo YAML fixtures.

## 2) Inputs / Outputs

Inputs:

- llhttp root: `$LLHTTP` or first positional arg
- output root: `$OUTPUT` or second positional arg

Source markdown directories:

- `$LLHTTP/test/request`
- `$LLHTTP/test/response`

Destination fixture directories:

- `$OUTPUT/tests/fixtures/llhttp/requests`
- `$OUTPUT/tests/fixtures/llhttp/responses`

## 3) Invocation

- `LLHTTP=<llhttp-root> OUTPUT=<output-root> node parser/tools/import-llhttp-tests.js`
- `node parser/tools/import-llhttp-tests.js <llhttp-root> <output-root>`
- Typical local usage from `parser/`: `node tools/import-llhttp-tests.js /Volumes/DATI/Users/Shogun/Programmazione/OSS/llhttp .`

## 4) High-level Pipeline

1. Validate roots.
2. Wipe and recreate `$OUTPUT/tests/fixtures/llhttp`.
3. Parse markdown tests from request and response directories.
4. Build fixture payload with markdown-derived data (`name`, `source`, `input`, `llhttp`).
5. Write temporary fixture adding `events: []` (compatibility shim for `cargo --generate`).
6. Run cargo generator for that exact fixture path.
7. Parse cargo stdout YAML snippet after `---`.
8. Rewrite same fixture with final shape and `output` inserted.

## 5) Markdown Parsing Rules

- `##` and `###` headings define test naming hierarchy.
- A case is emitted only when both `http` and `log` code blocks are present.
- `source` metadata is captured from heading:
  - `source.path`: markdown path relative to llhttp root (e.g. `test/request/sample.md`)
  - `source.line`: 1-based line number of the defining heading node
- No lenient/comment filter is applied.
- No HTTP/1.0 or RTSP/1.0 skip filter is applied.

## 6) Naming Strategy

- Display name:
  - H2 only -> `Title Case(H2)`
  - H2 + H3 -> `H2 / Title Case(H3)`
- Filename:
  - slugify heading parts, join with `-`, extension `.yml`
  - collisions resolved with `-2`, `-3`, ...

## 8) Fixture Schema (Final)

```yaml
---
name: 'Simple Request'
checked: false
source:
  path: 'test/request/sample.md'
  line: 6
input:
  - 'OPTIONS /url HTTP/1.1'
  - 'Header1: Value1'
  - ''
  - ''
llhttp:
  - 'off=0 message begin'
  - 'off=0 len=7 span[method]="OPTIONS"'
  - 'off=60 message complete'
  - ''
output:
  # parsed from cargo generator stdout (after ---)
```

Notes:

- `checked` is always hardcoded to `false`.
- `input` is verbatim markdown `http` block split with `input.split('\n')`.
- `llhttp` is the original `log` block split with `log.split('\n')`.
- There is no parsed/normalized llhttp-event structure in the fixture anymore.

## 7) Cargo Output Injection

For each generated fixture file, run:

- `cargo run --example llhttp -- --generate <request|response> <fixture-relative-path>`

Implementation details:

- command is executed with `cwd = $OUTPUT`.
- fixture path passed to cargo is relative to `$OUTPUT`.
- script finds first `---` in stdout, takes text after it, trims, parses as YAML.
- parsed object is stored under top-level `output` in final fixture.
- if cargo fails or snippet cannot be parsed, importer exits via `fail(...)`.

Compatibility shim detail:

- temporary write includes `events: []` only to satisfy current Rust loader expectation during `--generate`.
- final write removes that temporary field and keeps only the final schema.

## 8) YAML Serialization

Serialization uses npm `yaml` package (`YAML.stringify`) with:

- `lineWidth: 0`
- `defaultStringType: 'QUOTE_SINGLE'`
- `defaultKeyType: 'PLAIN'`

And prepends explicit document marker:

- `---`

## 9) Dependencies

`parser/tools/package.json` dependencies include:

- `remark-parse`
- `unified`
- `yaml`

## 10) Validation / Recommended Run Sequence

Minimum validation:

- `node --check tools/import-llhttp-tests.js`

Recommended:

1. Syntax check.
2. Run importer.
3. Inspect git diff under `parser/tests/fixtures/llhttp`.

## 11) Handoff Invariants

When continuing work, preserve these unless explicitly changed:

- top-level keys include `name`, `checked`, `source`, `input`, `llhttp`, `output`
- `checked` stays hardcoded to `false`
- `source.line` comes from heading definition line
- `llhttp` remains raw log lines (no event parsing)
- cargo `--generate` is executed per fixture and merged into `output`
- fixture root is deleted/recreated before generation
