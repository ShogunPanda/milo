# Rust API

## Constants

The crate exports several constants (`*` is used to denote a family prefix):

- `DEBUG`: If the debug informations are enabled or not.
- `MESSAGE_TYPE_*`: The type of the parser: it can autodetect (default) or only parse requests or response.
- `ERROR_*`: An error code.
- `METHOD_*`: An HTTP/RTSP request method.
- `CONNECTION_*`: A `Connection` header value.
- `CALLBACK_*`: A parser callback.
- `STATE_*`: A parser state.

## `Flags`

A struct representing the current compile flags of Milo:

- `debug`: If the debug informations are enabled or not.

## `Callback`

All callback in Milo have the following signature (`Callback`):

```rust
type Callback = fn (&mut Parser, usize, usize)
```

where the parameters have the following meaning:

1. The current parser.
2. The payload offset. Can be `0`.
3. The data length. Can be `0`.

If both offset and length are `0`, it means the callback has no payload associated.

## `ParserCallbacks`

A struct representing the callbacks for a parser.

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

- `mode` (`usize`): The current parser mode. Can be `MESSAGE_TYPE_AUTODETECT`, `MESSAGE_TYPE_REQUEST` or `MESSAGE_TYPE_RESPONSE`,
- `paused` (`bool`): If the parser is paused.
- `manage_unconsumed` (`bool`): If the parser should automatically copy and prepend unconsumed data.
- `continue_without_data` (`bool`): If the next execution of the parse loop should execute even if there is no more data.
- `is_connect` (`bool`): If the current request used `CONNECT` method.
- `skip_body` (`bool`): If the parser should skip the body.
- `owner` (`*mut c_void`): The context of this parser. Use is reserved to the developer.
- `state` (`usize`): The current parser state.
- `position` (`usize`): The current parser position in the slice in the current execution of `milo_parse`.
- `parsed` (`u64`): The total bytes consumed from this parser.
- `error_code` (`usize`): The parser error. By default is `ERROR_NONE`.
- `message_type` (`usize`): The current message type. Can be `MESSAGE_TYPE_REQUEST` or `MESSAGE_TYPE_RESPONSE`.
- `method` (`usize`): The current request method.
- `status` (`usize`): The current response status.
- `version_major` (`usize`): The current message HTTP version major version.
- `version_minor` (`usize`): The current message HTTP version minor version.
- `connection` (`usize`): The value for the connection header. Can be `CONNECTION_CLOSE`, `CONNECTION_UPGRADE` or `CONNECTION_KEEPALIVE` (which is the default when no header is set).
- `content_length` (`u64`): The value of the `Content-Length` header.
- `chunk_size` (`u64`): The expected length of the next chunk.
- `remaining_content_length` (`u64`): The missing data length of the body according to the `content_length` field.
- `remaining_chunk_size` (`u64`): The missing data length of the next chunk according to the `chunk_size` field.
- `has_content_length` (`bool`): If the current message has a `Content-Length` header.
- `has_chunked_transfer_encoding` (`bool`): If the current message has a `Transfer-Encoding` header.
- `has_upgrade` (`bool`): If the current message has a `Connection: upgrade` header.
- `has_trailers` (`bool`): If the current message has a `Trailers` header.
- `callbacks` (`ParserCallbacks`): The callbacks for the current parser.
- `error_description` (`*const c_uchar`): The parser error description.
- `error_description_len` (`usize`): The parser error description length.
- `unconsumed` (`*const c_uchar`): The unconsumed data from the previous execution of `parse` when `manage_unconsumed` is `true`.
- `unconsumed_len` (`usize`): The unconsumed data length from the previous execution of `parse` when `manage_unconsumed` is `true`.

All the fields **MUST** be considered readonly, with the following exceptions:

- `mode`
- `manage_unconsumed`
- `continue_without_data`
- `is_connect`
- `skip_body`
- `context`
- `callbacks`

### `Parser::new() -> Parser`

Creates a new parser.

### `Parser::parse(&mut self, data: *const c_uchar, limit: usize) -> usize`

Parses `data` up to `limit` characters.

It returns the number of consumed characters.

### `Parser::reset(&mut self, keep_parsed: bool)`

Resets a parser. The second parameters specifies if to also reset the
parsed counter.

The following fields are not modified:

- `position`
- `context`
- `mode`
- `manage_unconsumed`
- `continue_without_data`
- `context`

### `Parser::clear(&mut self)`

Clears all values about the message in the parser.

The connection and message type fields are not cleared.

### `Parser::pause(&mut self)`

Pauses the parser. The parser will have to be resumed via `Parser::resume`.

### `Parser::resume(&mut self)`

Resumes the parser.

### `Parser::finish(&mut self)`

Marks the parser as finished. Any new data received via `parse` will
put the parser in the error state.

### `Parser::move_to(&mut self, state: usize, advance: usize) -> usize`

Moves the parsers to a new state and marks a certain number of characters as used.

This is meant to internal use.

### `Parser::fail(&mut self, code: usize, description: &str) -> usize`

Marks the parsing a failed, setting a error code and and error message.

It always returns zero for internal use.

### `Parser::state_str(&self) -> &str`

Returns the current parser's state as string.

### `Parser::error_code_str(&self) -> &str`

Returns the current parser's error state as string.

### `Parser::error_description_str(&self) -> &str`

Returns the current parser's error descrition.

## `flags() -> Flags`

Returns a struct representing the current compile flags of Milo.

## `milo_noop(_parser: &Parser, _data: *const c_uchar, _len: usize)`

A callback that simply returns `0`.

Use this callback as pointer when you want to remove a callback from the parser.

## Enums

All the enums below implement `TryFrom<usize>` and `Into<&str>` traits and have the `as_str` method.

### `MessageTypes`

An enum listing all possible message types.

### `Errors`

An enum listing all possible parser errors.

### `Methods`

An enum listing all possible HTTP/RTSP methods.

### `Connections`

An enum listing all possible connection (`Connection` header value) types.

### `States`

An enum listing all possible parser states.

### `States`

An enum listing all possible parser callbacks.

## C++ public interface

The following functions are defined to allow Rust to work in a C++ environment.

While you can use these functions within Rust, it makes little sense as they only call the corresponding method of the parser passed as first argument.

### `CStringWithLength`

A struct representing a string containing the following fields:

- `ptr` (`*const c_uchar`): The string data pointer.
- `len` (`usize`): The string length.

### `milo_flags() -> Flags`

Returns a struct representing the current compile flags of Milo.

### `milo_free_string(s: CStringWithLength)`

Release memory from a string previously obtained from other APIs.

**By convention, all Milo's Rust function which ends in `_string` and that do not belong to a struct implementation MUST have their value freed up with this function when done.**

### `milo_create() -> *mut Parser`

Creates a new parser.

### `milo_destroy(ptr: *mut Parser)`

Destroys a parser.

### `milo_parse(parser: *mut Parser, data: *const c_uchar, limit: usize) -> usize`

Parses `data` up to `limit` characters.

It returns the number of consumed characters.

### `milo_reset(parser: *mut Parser, keep_parsed: bool)`

Resets a parser. The second parameters specifies if to also reset the
parsed counter.

The following fields are not modified:

- `position`
- `context`
- `mode`
- `manage_unconsumed`
- `continue_without_data`
- `context`

### `milo_clear(parser: *mut Parser)`

Clears all values about the message in the parser.

The connection and message type fields are not cleared.

### `milo_pause(parser: *mut Parser)`

Pauses the parser. The parser will have to be resumed via `milo::milo_resume`.

### `milo_resume(parser: *mut Parser)`

Resumes the parser.

### `milo_finish(parser: *mut Parser)`

Marks the parser as finished. Any new invocation of `milo_parse` will put the parser in the error state.

### `milo_fail(parser: *mut Parser, code: usize, description: CStringWithLength)`

Marks the parsing a failed, setting a error code and and error message.

### `milo_state_string(parser: *mut Parser) -> *const c_uchar`

Returns the current parser's state as string.

**The returned value MUST be freed using `milo_free_string`.**

### `milo_error_code_string(parser: *mut Parser) -> *const c_uchar`

Returns the current parser's error state as string.

**The returned value MUST be freed using `milo_free_string`.**

### `milo_error_description_string(parser: *mut Parser) -> *const c_uchar`

Returns the current parser's error descrition.

**The returned value MUST be freed using `milo_free_string`.**
