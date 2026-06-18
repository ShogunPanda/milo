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
- `STATE_*`: A parser state.

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

#### `States`

An enum listing all possible parser states.

Access is supported from string constant or numeric value.

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

#### `parseWithError(parser, data, limit)`

Parses `data` up to `limit` characters.

It returns an object containing the number of consumed characters and whether the parser errored:

```javascript
{
  consumed: 0,
  errored: false
}
```

Internally this wraps the `parse_with_error` WebAssembly export, which returns a signed 32-bit integer: `consumed` on success, and `-(consumed + 1)` on error. The extra `+ 1` allows representing errors that happen after consuming zero bytes.

Since the raw return value is a signed 32-bit integer, `parseWithError` supports up to `2_147_483_646` consumed bytes per call when an error is reported.

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

#### `shouldContinueWithoutData(parser)`

Returns `true` if the next execution of the parse loop should execute even if there is no more data.

#### `isConnect(parser)`

Returns `true` if the current request used `CONNECT` method.

#### `shouldSkipBody(parser)`

Returns `true` if the parser should skip the body.

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

#### `getVersionMajor(parser)`

Returns the parser current message HTTP version major version.

#### `getVersionMinor(parser)`

Returns the parser current message HTTP version minor version.

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

#### `setActiveCallbacks(parser, value)`

Sets the active callback bitmask on the parser.

#### `setShouldManageUnconsumed(parser, value)`

Sets if the parser should automatically copy and prepend unconsumed data.

#### `setShouldContinueWithoutData(parser, value)`

Sets if the next execution of the parse loop should execute even if there is no more data.

#### `setShouldSkipBody(parser, value)`

Set if the parser should skip the body.

#### `setIsConnect(parser, value)`

Sets if the current request used the `CONNECT` method.

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
