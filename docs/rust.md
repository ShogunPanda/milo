# Rust API

## `no-copy` mode

By default, Milo buffers unconsumed data from parsing and automatically prepends it in the next invocation of the `parse` method.

If Milo is built with the `no-copy` feature (`--features milo/no-copy`), such behavior is disabled.

## Constants

The crate exports several constants (`*` is used to denote a family prefix):

- `DEBUG`: If the debug informations are enabled or not.
- `NO_COPY`: If the `no-copy` mode is enabled.
- `AUTODETECT`: Set the parser to autodetect if the next message is a request or a response.
- `REQUEST`: Set the parser to only parse requests.
- `RESPONSE`: Set the parser to only parse response.
- `CONNECTION_*`: A `Connection` header value.
- `ERROR_*`: An error code.
- `METHOD_*`: An HTTP/RTSP request method.
- `STATE_*`: A parser state.

## `Flags`

A struct representing the current compile flags of Milo:

- `debug`: If the debug informations are enabled or not.
- `all_callbacks`: If Milo will invoke all headers callbacks.

## `Callback`

All callback in Milo have the following signature (`Callback`):

```rust
type Callback = fn (&Parser, *const c_uchar, usize) -> isize
```

where the parameters have the following meaning:

1. The current parser.
2. A data pointer. Can be `NULL` (and in that case the next parameter will be 0).
3. The data length. Can be `0` (and in that case the previous parameter will be `NULL`).

If Milo is built in `no-copy` mode (see above) then the callbacks signature changes as follows:

```rust
type Callback = fn (&Parser, usize, usize) -> isize
```

where the parameters have the following meaning:

1. The current parser.
2. The offset (relative to the last data passed to `milo_parse`) where the current payload starts.
3. The data length. Can be `0` (and in that case the previous parameter will be `0`).

In both cases the return value must be `0` in case of success, any other value will halt the parser in error state.

## `Callbacks`

A struct representing the callbacks for a parser. Each callback is wrapped in a `Cell`.

Here's the list of supported callbacks:

- `before_state_change`: Invoked before the parser change its state. _Only invoked in debug mode_.
- `after_state_change`: Invoked after the parser change its state. _Only invoked in debug mode_.
- `on_error`: Invoked after the parsing fails.
- `on_finish`: Invoked after the parser is marked as finished.
- `on_message_start`: Invoked after a new message starts.
- `on_message_complete`: Invoked after a message finishes.
- `on_request`: Invoked after the message is identified as a request.
- `on_response`: Invoked after the message is identified as a response.
- `on_reset`: Invoked after the parser is reset (either manually or after parsing a new message except the first one).
- `on_method`: Invoked after the HTTP method has been parsed.
- `on_url`: Invoked after the request URL has been parsed.
- `on_protocol`: Invoked after the request or response protocol has been parsed.
- `on_version`: Invoked after the request or response version has been parsed.
- `on_status`: Invoked after the response status has been parsed.
- `on_reason`: Invoked after the response status reason has been parsed.
- `on_header_name`: Invoked after a new header name has been parsed.
- `on_header_value`: Invoked after a new header value has been parsed.
- `on_headers`: Invoked after headers are completed.
- `on_connect`: Invoked in `CONNECT` requests after headers have been completed.
- `on_upgrade`: Invoked after a connection is upgraded via a `Connection: upgrade` request header.
- `on_chunk_length`: Invoked after a new chunk length has been parsed.
- `on_chunk_extension_name`: Invoked after a new chunk extension name has been parsed.
- `on_chunk_extension_value`: Invoked after a new chunk extension value has been parsed.
- `on_chunk`: Invoked after new chunk data is received.
- `on_data`: Invoked after new body data is received (either chunked or not).
- `on_body`: Invoked after the body has been parsed. Note that this has no data attached so `on_data` must be used to save the body.
- `on_trailer_name`: Invoked after a new trailer name has been parsed.
- `on_trailer_value`: Invoked after a new trailer value has been parsed.
- `on_trailers`: Invoked after trailers are completed.

If you want to remove a previously set callback, you can use the `milo_noop` function also exported by this crate.

## `Parser`

A struct representing a parser. It has the following fields:

- `owner` (`Cell<*mut c_void>`): The owner of this parser. Use is reserved to the developer.
- `state` (`Cell<u8>`): The current parser state.
- `position` (`Cell<usize>`): The current parser position in the slice in the current execution of `milo_parse`.
- `parsed` (`Cell<u64>`): The total bytes consumed from this parser.
- `paused` (`Cell<bool>`): If the parser is paused.
- `error_code` (`Cell<u8>`): The parser error. By default is `ERROR_NONE`.
- `error_description` (`Cell<*const c_uchar>`): The parser error description.
- `error_description_len` (`Cell<usize>`): The parser error description length.
- `unconsumed` (`Cell<*const c_uchar>`): The unconsumed data from the previous execution of `milo_parse`. _This is not available in `no-copy` mode (see above)_.
- `unconsumed_len` (`Cell<usize>`): The unconsumed data length from the previous execution of `milo_parse`. _This is not available in `no-copy` mode (see above)_.
- `id` (`Cell<u8>`): The current parser ID. Use is reserved to the developer.
- `mode` (`Cell<u8>`): The current parser mode. Can be `AUTODETECT`, `REQUEST` or `RESPONSE`,
- `continue_without_data` (`Cell<bool>`): If the next execution of the parse loop should execute even if there is no more data.
- `message_type` (`Cell<u8>`): The current message type. Can be `REQUEST` or `RESPONSE`.
- `is_connect` (`Cell<bool>`): If the current request used `CONNECT` method.
- `method` (`Cell<u8>`): The current request method as integer.
- `status` (`Cell<usize>`): The current response status.
- `version_major` (`Cell<u8>`): The current message HTTP version major version.
- `version_minor` (`Cell<u8>`): The current message HTTP version minor version.
- `connection` (`Cell<u8>`): The value for the connection header. Can be `CONNECTION_CLOSE`, `CONNECTION_UPGRADE` or `CONNECTION_KEEPALIVE` (which is the default when no header is set).
- `has_content_length` (`Cell<bool>`): If the current request has a `Content-Length` header.
- `has_chunked_transfer_encoding` (`Cell<bool>`): If the current request has a `Transfer-Encoding` header.
- `has_upgrade` (`Cell<bool>`): If the current request has a `Connection: upgrade` header.
- `has_trailers` (`Cell<bool>`): If the current request has a `Trailers` header.
- `content_length` (`Cell<u64>`): The value of the `Content-Length` header.
- `chunk_size` (`Cell<u64>`): The expected length of the next chunk.
- `remaining_content_length` (`Cell<u64>`): The missing data length of the body according to the `content_length` field.
- `remaining_chunk_size` (`Cell<u64>`): The missing data length of the next chunk according to the `chunk_size` field.
- `skip_body` (`Cell<bool>`): If the parser should skip the body.
- `callbacks` (`Callbacks`): The callbacks for the current parser.

Most of the fields **MUST** be considered readonly. The only exception are:

- `owner`
- `id`
- `mode`
- `is_connect`
- `skip_body`
- `callbacks`

The only persisted fields across resets are `owner`, `id`, `mode` and `continue_without_data`.

### `Parser::new() -> Parser`

Creates a new parser.

### `Parser::reset(&self, keep_parsed: bool)`

Resets a parser. The second parameters specifies if to also reset the parsed counter.

### `Parser::clear(&self)`

Clears all values in the parser.

Persisted fields, unconsumed data and the position are **NOT** cleared.

### `Parser::parse(&self, data: *const c_uchar, mut limit: usize) -> usize`

Parses `data` up to `limit` characters.

It returns the number of consumed characters.

### `Parser::pause(&self)`

Pauses the parser. The parser will have to be resumed via `Parser::resume`.

### `Parser::resume(&self)`

Resumes the parser.

### `Parser::finish(&self)`

Marks the parser as finished. Any new invocation of `Parser::parse` will put the parser in the error state.

### `Parser::state_string(&self) -> &str`

Returns the current parser's state as string.

### `Parser::error_code_string(&self) -> &str`

Returns the current parser's error state as string.

### `Parser::error_description_string(&self) -> &str`

Returns the current parser's error descrition.

### `flags() -> Flags`

Returns a struct representing the current compile flags of Milo.

## `milo_noop(_parser: &Parser, _data: *const c_uchar, _len: usize) -> isize`

A callback that simply returns `0`.

Use this callback as pointer when you want to remove a callback from the parser.

## Enums

All the enums below implement `TryFrom<u8>` and `Into<String>` traits and also have the `fn as_string() -> String` method.

### `MessageTypes`

An enum listing all possible message types.

### `Connections`

An enum listing all possible connection (`Connection` header value) types.

### `Methods`

An enum listing all possible HTTP/RTSP methods.

### `States`

An enum listing all possible parser states.

### `Errors`

An enum listing all possible parser errors.

## C++ public interface

The following functions are defined to allow Rust to work in a C++ environment.

While you can use these functions within Rust, it makes little sense as they only call the corresponding method of the parser passed as first argument.

### `milo_flags() -> Flags`

Returns a struct representing the current compile flags of Milo.

### `milo_free_string(s: *const c_uchar)`

Release memory from a string previously obtained from other APIs.

**By convention, all Milo's Rust function which ends in `_string` and that do not belong to a struct implementation MUST have their value freed up with this function when done.**

### `milo_create() -> *mut Parser`

Creates a new parser.

### `milo_destroy(ptr: *mut Parser)`

Destroys a parser.

### `milo_reset(parser: *const Parser, keep_parsed: bool)`

Resets a parser. The second parameters specifies if to also reset the parsed counter.

### `milo_parse(parser: *const Parser, data: *const c_uchar, limit: usize) -> usize`

Parses `data` up to `limit` characters.

It returns the number of consumed characters.

### `milo_pause(parser: *const Parser)`

Pauses the parser. The parser will have to be resumed via `milo::milo_resume`.

### `milo_resume(parser: *const Parser)`

Resumes the parser.

### `milo_finish(parser: *const Parser)`

Marks the parser as finished. Any new invocation of `milo_parse` will put the parser in the error state.

### `milo_state_string(parser: *const Parser) -> *const c_uchar`

Returns the current parser's state as string.

**The returned value MUST be freed using `milo_free_string`.**

### `milo_error_code_string(parser: *const Parser) -> *const c_uchar`

Returns the current parser's error state as string.

**The returned value MUST be freed using `milo_free_string`.**

### `milo_error_description_string(parser: *const Parser) -> *const c_uchar`

Returns the current parser's error descrition.

**The returned value MUST be freed using `milo_free_string`.**
