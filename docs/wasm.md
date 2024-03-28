# Using Milo in JavaScript via WebAssembly

## Install it

Use `npm` or similar:

```
npm install @perseveranza-pets/milo
```

# Javascript API

The module exports several constants (`*` is used to denote a family prefix):

- `FLAG_DEBUG`: If the debug informations are enabled or not.
- `MESSAGE_TYPE_*`: The type of the parser: it can autodetect (default) or only parse requests or response.
- `ERROR_*`: An error code.
- `METHOD_*`: An HTTP/RTSP request method.
- `CONNECTION_*`: A `Connection` header value.
- `CALLBACK_*`: A parser callback.
- `STATE_*`: A parser state.

## Callbacks handling

All callback in Milo have the following signature (TypeScript syntax):

```typescript
(parser: number, offset: number, length: number) => void
```

where the parameters have the following meaning:

1. The current parser.
2. The payload offset. Can be `0`.
3. The data length. Can be `0`.

If both offset and length are `0`, it means the callback has no payload associated.

## `MessageTypes`

An enum listing all possible message types.

Access is supported from string constant or numeric value.

## `Errors`

An enum listing all possible parser errors.

Access is supported from string constant or numeric value.

## `Methods`

An enum listing all possible HTTP/RTSP methods.

Access is supported from string constant or numeric value.

## `Connections`

An enum listing all possible connection (`Connection` header value) types.

Access is supported from string constant or numeric value.

## `Callbacks`

An enum listing all possible parser callbacks.

Access is supported from string constant or numeric value.

## `States`

An enum listing all possible parser states.

Access is supported from string constant or numeric value.

## `alloc`

Allocates a shared memory area with the WebAssembly instance which can be used to pass data to the parser.

**The returned value MUST be destroyed later using `dealloc`.**

## `dealloc(ptr)`

Deallocates a shared memory area created with `alloc`.

## `create`

Creates a new parser.

**The returned value MUST be destroyed later using `destroy`.**

## `destroy(parser)`

Destroys a parser.

## `parse(parser, data, limit)`

Parses `data` up to `limit` characters.

It returns the number of consumed characters.

## `reset(parser)`

Resets a parser. The second parameters specifies if to also reset the
parsed counter.

The following fields are not modified:

- `position`
- `context`
- `mode`
- `manage_unconsumed`
- `continue_without_data`
- `context`

## `clear(parser)`

Clears all values about the message in the parser.

The connection and message type fields are not cleared.

## `pause(parser)`

Pauses the parser. The parser will have to be resumed via `resume`.

## `resume(parser)`

Resumes the parser.

## `finish(parser)`

Marks the parser as finished. Any new invocation of `milo::milo_parse` will put the parser in the error state.

## `fail(parser, code, description)`

Marks the parsing a failed, setting a error code and and error message.

## `getMode(parser)`

Returns the parser mode.

## `isPaused(parser)`

Returns `true` if the parser is paused.

## `manageUnconsumed(parser)`

Returns `true` if the parser should automatically copy and prepend unconsumed data.

## `continueWithoutData(parser)`

Returns `true` if the next execution of the parse loop should execute even if there is no more data.

## `isConnect(parser)`

Returns `true` if the current request used `CONNECT` method.

## `skipBody(parser)`

Returns `true` if the parser should skip the body.

## `getState(parser)`

Returns the parser state.

## `getPosition(parser)`

Returns the parser position.

## `getParsed(parser)`

Returns the total bytes consumed from this parser.

## `getErrorCode(parser)`

Returns the parser error.

## `getMessageType(parser)`

Returns the parser current message type.

## `getMethod(parser)`

Returns the parser current request method.

## `getStatus(parser)`

Returns the parser current response status.

## `getVersionMajor(parser)`

Returns the parser current message HTTP version major version.

## `getVersionMinor(parser)`

Returns the parser current message HTTP version minor version.

## `getConnection(parser)`

Returns the parser value for the connection header.

## `getContentLength(parser)`

Returns the parser value of the `Content-Length` header.

## `getChunkSize(parser)`

Returns the parser expected length of the next chunk.

## `getRemainingContentLength(parser)`

Returns the parser missing data length of the body according to the `content_length` field.

## `getRemainingChunkSize(parser)`

Returns the parser missing data length of the next chunk according to to the `chunk_size` field.

## `hasContentLength(parser)`

Returns `true` if the parser the current message has a `Content-Length` header.

## `hasChunkedTransferEncoding(parser)`

Returns `true` if the parser the current message has a `Transfer-Encoding: chunked` header.

## `hasUpgrade(parser)`

Returns `true` if the parser the current message has a `Connection: upgrade` header.

## `hasTrailers(parser)`

Returns `true` if the parser the current message has a `Trailers` header.

## `getErrorDescription(parser)`

Returns the parser error description or `null`.

## `getCallbackError(parser)`

Returns the parser callback error or `null`.

## `setMode(parser, value)`

Sets the parser mode.

## `setManageUnconsumed(parser, value)`

Sets if the parser should automatically copy and prepend unconsumed data.

## `setContinueWithoutData(parser, value)`

Sets if the next execution of the parse loop should execute even if there is no more data.

## `setSkipBody(parser, value)`

Set if the parser should skip the body.

## `setIsConnect(parser, value)`

Sets if the current request used the `CONNECT` method.

## `setBeforeStateChange(parser, cb)`

Sets the parser `before_state_change` callback.

## `setAfterStateChange(parser, cb)`

Sets the parser `after_state_change` callback.

## `setOnError(parser, cb)`

Sets the parser `on_error` callback.

## `setOnFinish(parser, cb)`

Sets the parser `on_finish` callback.

## `setOnMessageStart(parser, cb)`

Sets the parser `on_message_start` callback.

## `setOnMessageComplete(parser, cb)`

Sets the parser `on_message_complete` callback.

## `setOnRequest(parser, cb)`

Sets the parser `on_request` callback.

## `setOnResponse(parser, cb)`

Sets the parser `on_response` callback.

## `setOnReset(parser, cb)`

Sets the parser `on_reset` callback.

## `setOnMethod(parser, cb)`

Sets the parser `on_method` callback.

## `setOnUrl(parser, cb)`

Sets the parser `on_url` callback.

## `setOnProtocol(parser, cb)`

Sets the parser `on_protocol` callback.

## `setOnVersion(parser, cb)`

Sets the parser `on_version` callback.

## `setOnStatus(parser, cb)`

Sets the parser `on_status` callback.

## `setOnReason(parser, cb)`

Sets the parser `on_reason` callback.

## `setOnHeaderName(parser, cb)`

Sets the parser `on_header_name` callback.

## `setOnHeaderValue(parser, cb)`

Sets the parser `on_header_value` callback.

## `setOnHeaders(parser, cb)`

Sets the parser `on_headers` callback.

## `setOnConnect(parser, cb)`

Sets the parser `on_connect` callback.

## `setOnUpgrade(parser, cb)`

Sets the parser `on_upgrade` callback.

## `setOnChunkLength(parser, cb)`

Sets the parser `on_chunk_length` callback.

## `setOnChunkExtensionName(parser, cb)`

Sets the parser `on_chunk_extension_name` callback.

## `setOnChunkExtensionValue(parser, cb)`

Sets the parser `on_chunk_extension_value` callback.

## `setOnChunk(parser, cb)`

Sets the parser `on_chunk` callback.

## `setOnBody(parser, cb)`

Sets the parser `on_body` callback.

## `setOnData(parser, cb)`

Sets the parser `on_data` callback.

## `setOnTrailerName(parser, cb)`

Sets the parser `on_trailer_name` callback.

## `setOnTrailerValue(parser, cb)`

Sets the parser `on_trailer_value` callback.

## `setOnTrailers(parser, cb)`

Sets the parser `on_trailers` callback.
