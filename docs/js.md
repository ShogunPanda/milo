# Javascript API

## Regular API

### Callbacks handling

All callbacks in Milo have the following signature (TypeScript syntax):

```typescript
(parser: number, offset: number, length: number) => void
```

where the parameters have the following meaning:

1. The current parser.
2. The payload offset. Can be `0`.
3. The data length. Can be `0`.

If length is `0`, it means the callback has no payload associated.

Callbacks are dispatched only when enabled with `setActiveCallbacks`.

Callbacks are disabled by default.

### Constants

The module exports several constants (`*` is used to denote a family prefix):

- `ERROR_*`: An error code.
- `METHOD_*`: An HTTP request method.
- `CALLBACK_*`: A parser callback.
- `CALLBACK_ACTIVE_*`: Callback activation flags.
- `EVENT_*`: A parser event type.
- `EVENT_ACTIVE_*`: Event activation flags.
- `STATE_*`: A parser state.
- `PARSER_FIELD_*`: A WebAssembly parser field offset.

Internal generated lookup tables used by the parser are not exported from the WebAssembly package.

### Enumerations

#### `Errors`

An enum listing all possible parser errors.

Access is supported from string constant or numeric value.

#### `Methods`

An enum listing all possible HTTP methods recognized by Milo.

Access is supported from string constant or numeric value.

#### `Callbacks`

An enum listing all possible parser callbacks.

Access is supported from string constant or numeric value.

#### `CallbackActives`

An enum listing all possible parser callbacks bitmask.

Access is supported from string constant or numeric value.

#### `Events`

An enum listing all possible parser event types.

Access is supported from string constant or numeric value.

#### `EventActives`

An enum listing all possible parser event activation flags.

Access is supported from string constant or bigint value.

#### `States`

An enum listing all possible parser states.

Access is supported from string constant or numeric value.

#### `ParserFields`

An enum listing WebAssembly parser field offsets.

Access is supported from string constant or numeric value.

### Parser Fields

`ParserFields` contains byte offsets for reading parser fields directly from WebAssembly memory.

Use the offset with the parser pointer returned by `create()`:

```javascript
const milo = setup()
const parser = milo.create()

const status = new DataView(milo.memory.buffer).getUint32(parser + milo.ParserFields.STATUS, true)
const paused = new Uint8Array(milo.memory.buffer)[parser + milo.ParserFields.PAUSED] !== 0

milo.destroy(parser)
```

Read fields with the matching WebAssembly representation:

- `bool` and `u8`: `Uint8Array`
- `u16`: `DataView#getUint16(..., true)`
- `u32` and `usize`: `DataView#getUint32(..., true)`
- `u64`: `DataView#getBigUint64(..., true)`
- pointers: `DataView#getUint32(..., true)`

`ParserFields.ERROR_DESCRIPTION` points to an inline `Uint8Array` buffer of 255 bytes inside the parser. It is always NIL-terminated. `ParserFields.ERROR_DESCRIPTION_LEN` is a `u8` length that excludes the terminator; error descriptions are clamped to 254 bytes.

Prefer the regular getters when available. `ParserFields` is intended for advanced WebAssembly integrations that need direct memory access.

### Events

Events are parser-owned records written to the parser event buffer during parsing. They are disabled by default. Enable them with `setActiveEvents(parser, mask)` using one or more `EVENT_ACTIVE_*` constants.

Callbacks are replayed from the same event buffer. Calling `setActiveCallbacks(parser, mask)` also enables event emission for those callbacks, then callbacks are invoked in event order before `parse()` returns.

Read the event buffer pointer from `parser + ParserFields.EVENTS`, then drain records from that pointer. The event stream is terminated by `EVENT_END`. Do not rely on the internal buffer size; always stop reading at `EVENT_END`. Event payload integers are little-endian.

If an active event would exceed the internal event buffer, parsing stops before consuming the data that would have produced the event. This is not a parser error and does not pause the parser. Call `parse()` again after draining the event buffer.

### Body Payload Limit

`setMaxBodyPayload(parser, value)` limits how many body payload bytes a single `parse()` invocation can consume. The default value is `0`, which means unlimited.

When the limit is reached, `parse()` returns normally with a consumed byte count smaller than `limit` and leaves the remaining input unconsumed. This is not a parser error and does not pause the parser. The next `parse()` invocation continues from the same parser state.

The limit applies only to body payload bytes. Framing bytes such as chunk headers, chunk CRLFs, and trailers are not counted.

### Suspend After Headers

`setShouldSuspendAfterHeaders(parser, true)` makes `parse()` return after the final header terminator has been consumed and `on_headers` has been emitted. The parser is not paused; the next `parse()` invocation continues with body decision and body parsing.

#### Range events

Most events use this payload:

```text
u8  type
u32 at
u32 len
```

`type` is one of the `EVENT_*` constants. `at` and `len` are relative to the last input passed to `parse()`. `len` can be `0`.

`EVENT_STATE_CHANGE` is debug-only and uses the same payload. For this event, `len` contains the new parser state id as a `u32`. Callback replay passes that value as the callback `size` argument.

#### Metadata events

`EVENT_HEADERS` uses this payload:

```text
u8  type
u32 at
u16 status_or_method
u8  should_keep_alive
u8  should_upgrade
u8  has_trailers
u8  body_kind
u64 content_length
```

`status_or_method` is the response status for responses and the request method for requests.

`body_kind` values are:

- `0`: `Content-Length`
- `1`: chunked transfer encoding
- `2`: no explicit body length

#### Error events

`EVENT_ERROR` uses this payload:

```text
u8  type
u32 at
u8  error_code
```

#### Reading events

```javascript
function readEvents (milo, parser) {
  const memory = milo.memory.buffer
  const eventsPtr = new DataView(memory).getUint32(parser + milo.ParserFields.EVENTS, true)
  const events = new Uint8Array(memory, eventsPtr)
  const view = new DataView(memory, eventsPtr)
  const decoded = []
  let cursor = 0

  for (;;) {
    const type = events[cursor]

    if (type === milo.EVENT_END) {
      break
    }

    if (type === milo.EVENT_ERROR) {
      decoded.push({ type, at: view.getUint32(cursor + 1, true), errorCode: events[cursor + 5] })
      cursor += 6
    } else if (type === milo.EVENT_HEADERS) {
      decoded.push({
        type,
        at: view.getUint32(cursor + 1, true),
        statusOrMethod: view.getUint16(cursor + 5, true),
        shouldKeepAlive: events[cursor + 7] !== 0,
        shouldUpgrade: events[cursor + 8] !== 0,
        hasTrailers: events[cursor + 9] !== 0,
        bodyKind: events[cursor + 10],
        contentLength: view.getBigUint64(cursor + 11, true)
      })
      cursor += 19
    } else {
      decoded.push({ type, at: view.getUint32(cursor + 1, true), len: view.getUint32(cursor + 5, true) })
      cursor += 9
    }
  }

  return decoded
}
```

### Methods

#### `setup`

Create a new milo module instance. Note that this is not a parser yet.

The method accepts a single object containing one or more of the following callbacks:

- `on_state_change`
- `on_error`
- `on_finish`
- `on_message_start`
- `on_message_complete`
- `on_request`
- `on_response`
- `on_reset`
- `on_method`
- `on_url`
- `on_protocol`
- `on_version`
- `on_status`
- `on_reason`
- `on_header_name`
- `on_header_value`
- `on_headers`
- `on_connect`
- `on_upgrade`
- `on_chunk_length`
- `on_chunk_extension_name`
- `on_chunk_extension_value`
- `on_chunk`
- `on_body`
- `on_data`
- `on_trailer_name`
- `on_trailer_value`
- `on_trailers`

Callbacks are disabled by default and must be enabled with `setActiveCallbacks` using one of the `CALLBACK_ACTIVE_*` constants.

The return object will be a milo module instance which can be use to create and manage parsers.

The object supports the methods below.

#### `alloc`

Allocates a shared memory area with the WebAssembly instance which can be used to pass data to the parser.

**The returned value MUST be destroyed later using `dealloc`.**

#### `dealloc(ptr)`

Deallocates a shared memory area created with `alloc`.

#### `create`

Creates a new parser.

**The returned value MUST be destroyed later using `destroy`.**

#### `destroy(parser)`

Destroys a parser.

#### `parse(parser, data, limit)`

Parses `data` up to `limit` characters.

It returns the number of consumed characters.

#### `reset(parser)`

Resets a parser. The second parameters specifies if to also reset the
parsed counter.

The following fields are not modified:

- `position`
- `context`
- `isAutodetect`
- `is_request`
- `manage_unconsumed`
- `continue_without_data`
- `debug`
- `max_start_line_length`
- `max_header_length`
- `context`
- `active_callbacks`
- `callbacks`

#### `clear(parser)`

Clears all values about the message in the parser.

The configured message type fields are not cleared.

#### `pause(parser)`

Pauses the parser. The parser will have to be resumed via `resume`.

#### `resume(parser)`

Resumes the parser.

#### `complete(parser)`

Completes the current message without consuming more input.

This emits normal completion events and performs the same completion transition
used by `parse`. It is valid only while the parser is in `BODY_DECISION`,
`TUNNEL`, `BODY_VIA_CONTENT_LENGTH`, `BODY_WITH_NO_LENGTH`, `CHUNK_HEADER`, or
`TRAILER`. Other states fail with `ERROR_UNEXPECTED_STATE`.

#### `finish(parser)`

Marks the parser as finished. Any new invocation of `parse` will put the parser in the error state.

#### `fail(parser, code, description)`

Marks the parsing a failed, setting a error code and and error message.

#### `hasDebug()`

Returns `true` if debug informations are available in this build.

#### `isAutodetect(parser)`

Returns `true` if the parser autodetects requests and responses.

#### `isRequest(parser)`

Returns `true` if the configured or detected message type is a request.

#### `isDebug(parser)`

Returns `true` if debug tracing is enabled for this parser.

The flag only affects tracing in debug-enabled builds.

#### `isPaused(parser)`

Returns `true` if the parser is paused.

#### `shouldManageUnconsumed(parser)`

Returns `true` if the parser should automatically copy and prepend unconsumed data.

#### `getMaxStartLineLength(parser)`

Returns the parser maximum allowed request/status line length.

Default is `8192`.

#### `getMaxHeaderLength(parser)`

Returns the parser maximum allowed header length.

Default is `8192`.

#### `getMaxBodyPayload(parser)`

Returns the maximum body payload bytes consumed by a single `parse()` invocation.

Default is `0`, which means unlimited.

#### `shouldContinueWithoutData(parser)`

Returns `true` if the next execution of the parse loop should execute even if there is no more data.

#### `isConnect(parser)`

Returns `true` if the current request used `CONNECT` method.

#### `shouldSkipBody(parser)`

Returns `true` if the parser should skip the body.

#### `shouldSuspendAfterHeaders(parser)`

Returns `true` if parsing should return after headers have completed.

#### `getState(parser)`

Returns the parser state.

#### `getPosition(parser)`

Returns the parser position.

#### `getParsed(parser)`

Returns the total bytes consumed from this parser.

#### `getErrorCode(parser)`

Returns the parser error.

#### `getMethod(parser)`

Returns the parser current request method.

#### `getStatus(parser)`

Returns the parser current response status.

#### `hasConnectionClose(parser)`

Returns `true` if the current message has a `Connection: close` token.

#### `hasConnectionUpgrade(parser)`

Returns `true` if the current message has a `Connection: upgrade` token.

#### `getContentLength(parser)`

Returns the parser value of the `Content-Length` header.

#### `getChunkSize(parser)`

Returns the parser expected length of the next chunk.

#### `getRemainingContentLength(parser)`

Returns the parser missing data length of the body according to the `content_length` field.

#### `getRemainingChunkSize(parser)`

Returns the parser missing data length of the next chunk according to to the `chunk_size` field.

#### `hasContentLength(parser)`

Returns `true` if the current message has a `Content-Length` header.

#### `hasTransferEncoding(parser)`

Returns `true` if the current message has a `Transfer-Encoding` header.

#### `hasChunkedTransferEncoding(parser)`

Returns `true` if the current message is using chunked encoding.

#### `hasUpgrade(parser)`

Returns `true` if the current message has an `Upgrade` header.

#### `hasTrailers(parser)`

Returns `true` if the current message has a `Trailer` header.

#### `getErrorDescription(parser)`

Returns the parser error description or `null`.

#### `setShouldAutodetect(parser, value)`

Sets if the parser should autodetect requests and responses.

#### `setIsRequest(parser, value)`

Sets the parser message type when `autodetect` is disabled.

#### `setMaxStartLineLength(parser, value)`

Sets the parser maximum allowed request/status line length.

Defaults to `8192` in a new parser.

#### `setMaxHeaderLength(parser, value)`

Sets the parser maximum allowed header length.

Defaults to `8192` in a new parser.

#### `setMaxBodyPayload(parser, value)`

Sets the maximum body payload bytes consumed by a single `parse()` invocation. Use `0` for unlimited.

#### `setActiveCallbacks(parser, value)`

Sets the active callback bitmask on the parser.

#### `setActiveEvents(parser, value)`

Sets the active event bitmask on the parser.

#### `setShouldManageUnconsumed(parser, value)`

Sets if the parser should automatically copy and prepend unconsumed data.

#### `setShouldSuspendAfterHeaders(parser, value)`

Sets if parsing should return after headers have completed.

#### `setShouldContinueWithoutData(parser, value)`

Sets if the next execution of the parse loop should execute even if there is no more data.

#### `setShouldSkipBody(parser, value)`

Set if the parser should skip the body.

#### `setIsConnect(parser, value)`

Sets if the current request used the `CONNECT` method.

#### `setDebug(parser, value)`

Sets if debug tracing is enabled for this parser.

The flag only affects tracing in debug-enabled builds.

## Simple API

A preconfigured module instance exported as `simple`.

It is equivalent to calling `setup(...)` with all callbacks wired to collect parser spans automatically.

Differences from a plain `setup()` instance:

- `create()` enables all callbacks automatically (`CALLBACK_ACTIVE_ALL`).
- `destroy(parser)` also clears collected spans for that parser.
- A `spans` object is exposed on the module, keyed by parser id.
- Every callback appends `[callbackType, offset, length]` to `spans[parser]`.

This API is useful when you want a minimal integration that records parser events without manually providing callback functions.

Example:

```javascript
import { simple } from '@perseveranza-pets/milo'

const milo = simple()
const message = Buffer.from('HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc')
const ptr = milo.alloc(message.length)
const parser = milo.create()

const buffer = Buffer.from(milo.memory.buffer, ptr, message.length)
buffer.set(message, 0)
milo.parse(parser, ptr, message.length)

for (const [type, at, len] of milo.spans[parser]) {
  console.log(`[${at.toString().padStart(3, ' ')}, ${len.toString().padStart(3, ' ')}] -> ${milo.Callbacks[type]}`)
}

milo.destroy(parser)
milo.dealloc(ptr, message.length)
```

will print

```
[  0,   0] -> ON_RESPONSE
[  0,   0] -> ON_MESSAGE_START
[  0,   4] -> ON_PROTOCOL
[  5,   3] -> ON_VERSION
[  9,   3] -> ON_STATUS
[ 13,   2] -> ON_REASON
[ 17,  14] -> ON_HEADER_NAME
[ 33,   1] -> ON_HEADER_VALUE
[ 38,   0] -> ON_HEADERS
[ 38,   3] -> ON_DATA
[ 38,   0] -> ON_BODY
[ 41,   0] -> ON_MESSAGE_COMPLETE
[ 41,   0] -> ON_RESET
```
