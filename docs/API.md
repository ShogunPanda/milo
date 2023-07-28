# C++ API

The file `milo.h` defines several constants (`*` is used to denote a family prefix):

- `MILO_VERSION_MAJOR`: The current milo complete version as a string.
- `MILO_VERSION_MAJOR`: The current milo major version.
- `MILO_VERSION_MINOR`: The current milo minor version.
- `MILO_VERSION_PATCH` The current milo patch version.
- `AUTODETECT`: Set the parser to autodetect if the next message is a request or a response.
- `REQUEST`: Set the parser to only parse requests.
- `RESPONSE`: Set the parser to only parse response.
- `CONNECTION_*`: A `Connection` header value.
- `ERROR_*`: An error code.
- `METHOD_*`: An HTTP/RTSP request method.
- `STATE_*`: A parser state.

## `milo::Error`

A enum representing a possible error code.

## `milo::Method`

A enum representing a possible HTTP/RTSP method.

## `milo::State`

A enum representing a possible parser state.

## `milo::Callback`

All callback in milo have the following signature (`Callback`):

```cpp
intptr_t(*)(milo::Parser*, const unsigned char*, uintptr_t)
```

where the parameters have the following meaning:

1. The current parser.
2. A data slice. Can be `NULL` (and in that case the next parameter will be 0).
3. The data length. Can be `0` (and in that case the previous parameter will be `NULL`).

## `milo::Callbacks`

A struct representing the callbacks for a parser. Here's the list of supported callbacks:

- `before_state_change`: Invoked before the parser change its state. _Only available in debug mode_.
- `after_state_change`: Invoked after the parser change its state. _Only available in debug mode_.
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
- `on_data`: Invoked after new body data is received (either chunked or not).
- `on_body`: Invoked after the body has been parsed. Note that this has no data attached so `on_data` must be used to save the body.
- `on_trailer_name`: Invoked after a new trailer name has been parsed.
- `on_trailer_value`: Invoked after a new trailer value has been parsed.
- `on_trailers`: Invoked after trailers are completed.

If you want to remove a previously set callback, you can use `milo::milo_noop`.

## `milo::Parser`

A struct representing a parser. It has the following fields:

- `owner` (`void*`): The owner of this parser. Use is reserved to the developer.
- `state` (`State`): The current parser state.
- `position` (`uintptr_t`): The current parser position.
- `paused` (`bool`): If the parser is paused.
- `error_code` (`Error`): The parser error. By default is `milo::Error::NONE`.
- `error_description` (`const unsigned char*`): The parser error description.
- `error_description_len` (`uintptr_t`): The parser error description length.
- `unconsumed` (`const unsigned char*`): The unconsumed data from the previous execution of `milo::milo_parse`.
- `unconsumed_len` (`uintptr_t`): The unconsumed data length from the previous execution of `milo::milo_parse`.
- `id` (`intptr_t`): The current parser ID. Use is reserved to the developer.
- `mode` (`intptr_t`): The current parser mode. Can be `milo::AUTODETECT`, `milo::REQUEST` or `milo::RESPONSE`,
- `continue_without_data` (`intptr_t`): It is set to `1` if the next execution of the parse loop should execute even if there is no more data.
- `message_type` (`intptr_t`): The current message type. Can be `milo::REQUEST` or `milo::RESPONSE`.
- `is_connect_request` (`intptr_t`): If the current request used `CONNECT` method.
- `method` (`intptr_t`): The current request method as integer.
- `status` (`intptr_t`): The current response status.
- `version_major` (`intptr_t`): The current message HTTP version major version.
- `version_minor` (`intptr_t`): The current message HTTP version minor version.
- `connection` (`intptr_t`): The value for the connection header. Can be `milo::CONNECTION_CLOSE`, `milo::CONNECTION_UPGRADE` or `milo::CONNECTION_KEEPALIVE` (which is the default when no header is set).
- `has_content_length` (`intptr_t`): It is set to `1` if the current request has a `Content-Length` header.
- `has_chunked_transfer_encoding` (`intptr_t`): It is set to `1` if the current request has a `Transfer-Encoding` header.
- `has_upgrade` (`intptr_t`): It is set to `1` if the current request has a `Connection: upgrade` header.
- `has_trailers` (`intptr_t`): It is set to `1` if the current request has a `Trailers` header.
- `content_length` (`intptr_t`): The value of the `Content-Length` header.
- `chunk_size` (`intptr_t`): The expected length of the next chunk.
- `remaining_content_length` (`intptr_t`): The missing data length of the body according to the `content_length` field.
- `remaining_chunk_size` (`intptr_t`): The missing data length of the next chunk according to the `chunk_size` field.
- `skip_body` (`intptr_t`): If the parser should skip the body.
- `callbacks` (`Callbacks`): The callbacks for the current parser.

Most of the fields **MUST** be considered readonly. The only exception are:

- `owner`
- `id`
- `mode`
- `skip_body`
- `callbacks`

The only persisted fields across resets are `owner`, `id`, `mode` and `continue_without_data`.

## `intptr_t milo_noop(Parser *_parser, const unsigned char *_data, uintptr_t _len)`

A callback that simply returns `0`.

Use this callback as pointer when you want to remove a callback from the parser.

## `void milo_free_string(const unsigned char *s)`

Release memory from a string previously obtained from other APIs.

**By convention, all milo's C++ function which ends in `_string` MUST have their value freed up with this function when done.**

## `Parser *milo_create()`

Creates a new parser.

**The returned value MUST be destroyed later using `milo_destroy`.**

## `void milo_destroy(Parser *ptr)`

Destroys a parser.

## `void milo_reset(Parser *parser, bool keep_position)`

Resets a parser. The second parameters specifies if to also reset the position counter.

## `uintptr_t milo_parse(Parser *parser, const unsigned char *data, uintptr_t limit)`

Parses `data` up to `limit` characters.

It returns the number of consumed characters.

## `void milo_pause(Parser *parser)`

Pauses the parser. The parser will have to be resumed via `milo::milo_resume`.

## `void milo_resume(Parser *parser)`

Resumes the parser.

## `void milo_finish(Parser *parser)`

Marks the parser as finished. Any new invocation of `milo::milo_parse` will put the parser in the error state.

## `const unsigned char *milo_state_string(Parser *parser)`

Returns the current parser's state as string.

**The returned value MUST be freed using `milo::milo_free_string`.**

## `const unsigned char *milo_error_code_string(Parser *parser)`

Returns the current parser's error state as string.

**The returned value MUST be freed using `milo::milo_free_string`.**

## `const unsigned char *milo_error_description_string(Parser *parser)`

Returns the current parser's error descrition.

**The returned value MUST be freed using `milo::milo_free_string`.**

# Rust API

The crate exports several constants (`*` is used to denote a family prefix):

- `AUTODETECT`: Set the parser to autodetect if the next message is a request or a response.
- `REQUEST`: Set the parser to only parse requests.
- `RESPONSE`: Set the parser to only parse response.
- `CONNECTION_*`: A `Connection` header value.
- `ERROR_*`: An error code.
- `METHOD_*`: An HTTP/RTSP request method.
- `STATE_*`: A parser state.

## `Error`

A enum representing a possible error code.

## `Method`

A enum representing a possible HTTP/RTSP method.

## `State`

A enum representing a possible parser state.

## `Callback`

All callback in milo have the following signature (`Callback`):

```rust
type Callback = fn (&mut Parser, *const c_uchar, usize) -> isize
```

where the parameters have the following meaning:

1. The current parser.
2. A data pointer. Can be `NULL` (and in that case the next parameter will be 0).
3. The data length. Can be `0` (and in that case the previous parameter will be `NULL`).

## `Callbacks`

A struct representing the callbacks for a parser. Here's the list of supported callbacks:

- `before_state_change`: Invoked before the parser change its state. _Only available in debug mode_.
- `after_state_change`: Invoked after the parser change its state. _Only available in debug mode_.
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
- `on_data`: Invoked after new body data is received (either chunked or not).
- `on_body`: Invoked after the body has been parsed. Note that this has no data attached so `on_data` must be used to save the body.
- `on_trailer_name`: Invoked after a new trailer name has been parsed.
- `on_trailer_value`: Invoked after a new trailer value has been parsed.
- `on_trailers`: Invoked after trailers are completed.

If you want to remove a previously set callback, you can use the `milo_noop` function also exported by this crate.

## `Parser`

A struct representing a parser. It has the following fields:

- `owner` (`*mut c_void`): The owner of this parser. Use is reserved to the developer.
- `state` (`State`): The current parser state.
- `position` (`usize`): The current parser position.
- `paused` (`bool`): If the parser is paused.
- `error_code` (`Error`): The parser error. By default is `Error::NONE`.
- `error_description` (`*const c_uchar`): The parser error description.
- `error_description_len` (`usize`): The parser error description length.
- `unconsumed` (`*const c_uchar`): The unconsumed data from the previous execution of `milo_parse`.
- `unconsumed_len` (`usize`): The unconsumed data length from the previous execution of `milo_parse`.
- `id` (`isize`): The current parser ID. Use is reserved to the developer.
- `mode` (`isize`): The current parser mode. Can be `AUTODETECT`, `REQUEST` or `RESPONSE`,
- `continue_without_data` (`isize`): It is set to `1` if the next execution of the parse loop should execute even if there is no more data.
- `message_type` (`isize`): The current message type. Can be `REQUEST` or `RESPONSE`.
- `is_connect_request` (`isize`): If the current request used `CONNECT` method.
- `method` (`isize`): The current request method as integer.
- `status` (`isize`): The current response status.
- `version_major` (`isize`): The current message HTTP version major version.
- `version_minor` (`isize`): The current message HTTP version minor version.
- `connection` (`isize`): The value for the connection header. Can be `CONNECTION_CLOSE`, `CONNECTION_UPGRADE` or `CONNECTION_KEEPALIVE` (which is the default when no header is set).
- `has_content_length` (`isize`): It is set to `1` if the current request has a `Content-Length` header.
- `has_chunked_transfer_encoding` (`isize`): It is set to `1` if the current request has a `Transfer-Encoding` header.
- `has_upgrade` (`isize`): It is set to `1` if the current request has a `Connection: upgrade` header.
- `has_trailers` (`isize`): It is set to `1` if the current request has a `Trailers` header.
- `content_length` (`isize`): The value of the `Content-Length` header.
- `chunk_size` (`isize`): The expected length of the next chunk.
- `remaining_content_length` (`isize`): The missing data length of the body according to the `content_length` field.
- `remaining_chunk_size` (`isize`): The missing data length of the next chunk according to the `chunk_size` field.
- `skip_body` (`isize`): If the parser should skip the body.
- `callbacks` (`Callbacks`): The callbacks for the current parser.

Most of the fields **MUST** be considered readonly. The only exception are:

- `owner`
- `id`
- `mode`
- `skip_body`
- `callbacks`

The only persisted fields across resets are `owner`, `id`, `mode` and `continue_without_data`.

### `Parser::new() -> Parser`

Creates a new parser.

### `Parser::reset(&mut self, keep_position: bool)`

Resets a parser. The second parameters specifies if to also reset the position counter.

### `Parser::clear(&mut self)`

Clears all values in the parser.

Persisted fields, unconsumed data and the position are **NOT** cleared.

### `Parser::parse(&mut self, data: *const c_uchar, mut limit: usize) -> usize`

Parses `data` up to `limit` characters.

It returns the number of consumed characters.

### `Parser::pause(&mut self)`

Pauses the parser. The parser will have to be resumed via `Parser::resume`.

### `Parser::resume(&mut self)`

Resumes the parser.

### `Parser::finish(&mut self)`

Marks the parser as finished. Any new invocation of `Parser::parse` will put the parser in the error state.

### `Parser::state_string(&mut self) -> String`

Returns the current parser's state as string.

### `Parser::error_code_string(&mut self) -> String`

Returns the current parser's error state as string.

### `Parser::error_description_string(&mut self) -> String`

Returns the current parser's error descrition.

## `milo_noop(_parser: &mut Parser, _data: *const c_uchar, _len: usize) -> isize`

A callback that simply returns `0`.

Use this callback as pointer when you want to remove a callback from the parser.

## C++ public interface

The following functions are defined to allow Rust to work in a C++ environment.

While you can use these functions within Rust, it makes little sense as they only call the corresponding method of the parser passed as first argument.

### `milo_free_string(s: *const c_uchar)`

Release memory from a string previously obtained from other APIs.

**By convention, all milo's Rust function which ends in `_string` and that do not belong to a struct implementation MUST have their value freed up with this function when done.**

### `milo_create() -> *mut Parser`

Creates a new parser.

### `milo_destroy(ptr: *mut Parser)`

Destroys a parser.

### `milo_reset(parser: *mut Parser, keep_position: bool)`

Resets a parser. The second parameters specifies if to also reset the position counter.

### `milo_parse(parser: *mut Parser, data: *const c_uchar, limit: usize) -> usize`

Parses `data` up to `limit` characters.

It returns the number of consumed characters.

### `milo_pause(parser: *mut Parser)`

Pauses the parser. The parser will have to be resumed via `milo::milo_resume`.

### `milo_resume(parser: *mut Parser)`

Resumes the parser.

### `milo_finish(parser: *mut Parser)`

Marks the parser as finished. Any new invocation of `milo_parse` will put the parser in the error state.

### `milo_state_string(parser: *mut Parser) -> *const c_uchar`

Returns the current parser's state as string.

**The returned value MUST be freed using `milo_free_string`.**

### `milo_error_code_string(parser: *mut Parser) -> *const c_uchar`

Returns the current parser's error state as string.

**The returned value MUST be freed using `milo_free_string`.**

### `milo_error_description_string(parser: *mut Parser) -> *const c_uchar`

Returns the current parser's error descrition.

**The returned value MUST be freed using `milo_free_string`.**
