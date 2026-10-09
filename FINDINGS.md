# Security findings

This document records the findings from a security review of Milo 0.9.0 at
commit `0cd8b81`. The review covered HTTP/1 framing, the Rust API, native and
WebAssembly bindings, callback dispatch, resource limits, and the release
pipeline.

Severity describes the worst realistic impact under the stated preconditions.
Items marked as integration boundaries are still important, but do not become
remote vulnerabilities unless an embedding application exposes the relevant
behavior.

## High severity

### SEC-01: The safe Rust API can invoke undefined behavior

`Parser::parse` is a safe method that accepts a raw pointer and length and
constructs a slice without validating the pointer. Allocation-owning fields,
including `events`, `unconsumed`, and `unconsumed_len`, are also public and can
be replaced from safe Rust before reset or destruction reconstructs and frees
them. Native helper functions similarly expose unchecked pointer operations to
safe Rust callers.

Safe Rust must not be able to cause invalid reads, writes, or frees. The safe
entry point should accept `&[u8]`; raw-pointer operations should be explicitly
unsafe; and ownership-bearing parser fields should be private or moved behind
an opaque ABI handle.

### SEC-02: Error-description truncation can create an invalid Rust string

Error descriptions are truncated to 254 bytes without preserving a UTF-8
character boundary. `error_description_str` then uses
`from_utf8_unchecked`. A valid description containing a multibyte character at
the truncation boundary can therefore produce an invalid `&str` from safe
Rust.

The native and WebAssembly failure entry points also trust caller bytes as
UTF-8. The JavaScript wrapper allocates `description.length` UTF-16 code units
rather than the encoded UTF-8 length and ignores partial `encodeInto` writes.
Truncation must preserve character boundaries, FFI bytes must be validated, and
the JavaScript wrapper must allocate the encoded byte length.

### SEC-03: Invalid Transfer-Encoding syntax can activate chunked framing

Transfer-Encoding list members that are not `chunked` are not validated using
the transfer-coding grammar. Inputs such as `bad@coding, chunked` and a trailing
empty member such as `chunked,` are accepted while chunked framing is enabled.

A Milo-based intermediary can therefore disagree with another HTTP parser
about whether the message is chunked. This is a request-smuggling risk when raw
messages pass through heterogeneous parsers. Every list member and parameter
must be parsed and validated before it influences framing.

### SEC-04: Callbacks can reenter or destroy a live parser

Callbacks run while the parser and its event buffer are still borrowed by the
outer parse operation. Native or JavaScript callbacks can call `parse`, `reset`,
`fail`, or `destroy` on that same parser. Destruction causes the outer replay
loop to continue through freed memory; mutation can replace the event batch
being replayed; and recursive parsing can exhaust the stack.

Parser mutation and destruction must be rejected or deferred while callback
replay is active. Replay should also use a bounded immutable snapshot.

### SEC-05: The release workflow executes mutable, unverified tools

Release jobs download the latest `cambi` and cargo-make binaries without an
immutable version or checksum, and download Binaryen without applying the
checksum already used by the Docker build. The credential-bearing release job
then executes these tools while preparing published artifacts. Cargo lockfiles
are also regenerated during publication rather than reviewed beforehand.

Release tools and actions should be pinned immutably, downloaded binaries
should be verified, reviewed lockfiles should be used with `--locked`, and
credential-free builds should be separated from publication.

## Medium severity

### SEC-06: Managed-input counts and callback ranges use undocumented coordinates

When `manage_unconsumed` is enabled, retained bytes and the current input are
combined into an internal buffer. The returned consumed count and callback
ranges refer to that aggregate, although the public documentation says they are
relative to the latest input. A small current buffer can therefore receive a
consumed count or callback offset beyond its bounds.

The API should report current-call consumption and provide an explicit backing
buffer for aggregate ranges, or otherwise distinguish retained and current
input coordinates.

### SEC-07: Event-buffer suspension commits header state non-atomically

Special-header state is updated before all events for the header are known to
fit in the event buffer. If the header-name event fits but the header-value
event does not, parsing returns without consuming the header while retaining
its semantic effects. Retrying the same valid Content-Length or chunked header
can then fail as a duplicate.

Event capacity must be reserved for the complete header operation before state
is committed, or parsing must commit temporary state and events atomically.

### SEC-08: Fragmented response autodetection permanently chooses request mode

Autodetection recognizes a response only when at least five bytes are already
available. A shorter prefix such as `H`, `HT`, or `HTTP` changes the parser to
request-line state. Even when managed input later supplies the complete
response, the parser does not reconsider that decision and rejects it.

An undecided state should retain or defer short prefixes until the message type
can be determined.

### SEC-09: Managed input retains an unbounded tunnel stream

Tunnel state intentionally consumes no further HTTP data. With
`manage_unconsumed` enabled, however, every subsequent tunnel byte is retained,
concatenated, and copied again on the next call. A CONNECT or Upgrade stream can
therefore cause unbounded memory growth and quadratic copying if an integration
continues passing tunnel data to Milo.

Tunnel data must not be retained by the HTTP parser.

### SEC-10: Byte-fragmented managed lines cause quadratic work

Every managed-input retry allocates and concatenates the complete retained
prefix, then scans it again. Sending a near-limit start line or header one byte
per call therefore performs quadratic copying and scanning. The per-line limit
bounds one occurrence, but an attacker can repeat many such headers.

Managed parsing should retain a growable buffer without rebuilding and
rescanning the complete prefix on every call.

### SEC-11: Chunked GET and HEAD bypass the documented body policy

The documented strict policy rejects request bodies on GET and HEAD, but the
implementation checks only for a positive Content-Length. Transfer-Encoding:
chunked bodies are accepted.

If the strict policy is retained, it must reject every body-framing mechanism
for GET and HEAD. Otherwise the documentation and downstream expectations must
be changed consistently.

### SEC-12: CONNECT enters tunnel mode before resolving declared body framing

CONNECT is moved to tunnel state before Content-Length or Transfer-Encoding
framing is processed. Another hop can consume a declared request body before
switching while Milo treats the same bytes as tunnel data.

Milo should either reject request-body framing on CONNECT or consume valid
framing before entering a pending tunnel state. Applications should explicitly
confirm a successful CONNECT response before relaying opaque data.

### SEC-13: Native callbacks use the Rust calling convention

The generated callback type is a Rust-ABI `fn(&mut Parser, usize, usize)`, while
C++ integrations install and invoke C/C++ functions through it. The calling
conventions happen to align on common current targets but this is not a Rust
ABI guarantee.

The ABI-facing callback type should be an `unsafe extern "C" fn` using raw
pointers. Rust-only ergonomic callbacks should use a separate interface.

### SEC-14: Event positions and lengths silently narrow to 32 bits

Event offsets and lengths are cast from `usize` to `u32`. A native parse range
larger than 4 GiB wraps before callback replay. Integrations can consequently
inspect or account for the wrong body range.

Large ranges should be split, rejected, or represented with a wider event
format.

### SEC-15: WebAssembly header metadata loses Content-Length precision

The WebAssembly `on_headers` callback converts a `u64` Content-Length to
JavaScript `f64`. Values above `2^53` lose integer precision even though the raw
event buffer and getter retain the original value.

The callback should use a BigInt-compatible `i64`, two 32-bit halves, or omit
the lossy number in favor of the exact getter/event value.

### SEC-16: There is no aggregate header-block budget

Start-line and header limits apply to individual incomplete lines. A message
can contain an unbounded number of individually valid headers, consuming CPU,
callback storage, and application memory. This also makes event-buffer edge
cases reachable with relatively small header lines.

Add configurable total header bytes and header-count limits, with conservative
defaults.

### SEC-17: The JavaScript simple API accumulates spans without a bound

`simple()` enables all callbacks and appends every event to a parser-specific
array until the parser is destroyed. A long-lived parser over pipelined or
chunked attacker traffic can therefore grow the JavaScript heap indefinitely.

Provide an explicit drain operation and an optional configurable cap.

### SEC-18: Partial start lines are accepted by finish()

`finish()` treats request-line and status-line states like an idle parser. A
partial line at EOF is moved to FINISH instead of producing
`UNEXPECTED_EOF`. This can bypass truncation detection and error accounting.

Only an untouched start state should finish cleanly; partial start lines should
fail.

### SEC-19: Explicit completion and failure paths do not replay callbacks

`parse()` invokes callbacks, but `complete()`, `finish()`, and direct `fail()`
can create events without replaying the corresponding callbacks. Callback-only
integrations can miss completion, reset, finish, or error notifications needed
for cleanup and accounting.

All public state-transition methods should define and consistently implement
callback delivery.

### SEC-20: Release tags are mutable and native assets lack attestations

The release workflow force-creates version tags and force-pushes all tags.
Native archives are uploaded without repository-provided digest manifests or
artifact attestations. A release rerun can therefore change the source behind a
version and consumers cannot verify native assets as strongly as npm packages
published with provenance.

Existing release tags should be immutable. Native artifacts should receive
SHA-256 manifests and GitHub/Sigstore attestations.

## Low severity and documented integration boundaries

### SEC-21: Forbidden trailer fields are accepted

Trailer parsing validates general field syntax but accepts framing, routing,
and authentication fields such as Content-Length, Transfer-Encoding, Host, and
Authorization. An application that merges trailers into its initial header map
can allow late values to override security decisions.

Milo should reject protocol-critical trailer names or provide a strict mode and
clear integration guidance.

### SEC-22: Host and request-target semantic validation require another layer

Host cardinality, absolute-form authority agreement, method-specific target
forms, CONNECT authority syntax, and percent-triplet validity are intentionally
outside Milo's current scope. Proxy and server users still need these checks to
prevent routing differentials, cache poisoning, and virtual-host access-control
bypass.

Provide an optional strict proxy mode or a first-party semantic validation
layer, while preserving the lower-level parser when required.

### SEC-23: The C++ reference uses fixed-size callback buffers

The C++ reference allocates fixed 1000-byte buffers and copies callback-sized
input without applying the buffer capacity. Reusing the example with larger
headers or body segments can overflow the heap buffer.

Construct strings directly from `(data, size)` or allocate `size + 1`, and add a
reference test with an 8192-byte field value.

## Existing protections verified during the review

Milo already rejects Content-Length combined with Transfer-Encoding, rejects
duplicate Content-Length fields, bounds numeric Content-Length and chunk-size
parsing, requires `chunked` to be unique and final, rejects bare CR/LF and
obs-fold, validates chunk-data CRLF, requires `Connection: upgrade`, and stops
accepting HTTP after `Connection: close`. Event writers reserve space for the
terminator and error record, SIMD scanners guard full-width loads, and retries
in error state do not grow retained input.

The Rust test suite passed in full during the review. Targeted probes also
reproduced SEC-02, SEC-03, SEC-07, SEC-08, SEC-09, and SEC-11.
