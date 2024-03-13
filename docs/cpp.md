# Linking Milo in C++

Milo release is composed by a header file `milo.h` and static library `libmilo.a`

The exact command to build milo is dependent on your compiler. For instance, for `clang` the command is the following:

```bash
clang++ -std=c++11 -I $MILO_DIR -o output $MILO_DIR/libmilo.a main.cc
```

where `$MILO_DIR` is the directory containing the `milo.h` and `libmilo.a` files.

When downloading Milo, you can choose between 4 different flavors:

1. `release/with-copy`: Release version with copying enabled.
2. `release/no-copy`: Release version with copying disabled.
3. `debug/with-copy`: Debug version with copying enabled.
4. `debug/no-copy`: Debug version with copying disabled.

## `with-copy` vs `no-copy` mode

The `with-copy` mode of Milo buffers unconsumed data from parsing and automatically prepends it in the next invocation of the `parse` method.

The `no-copy` mode has the behavior above disabled.

# C++ API

The file `milo.h` defines several constants (`*` is used to denote a family prefix):

- `MILO_VERSION_MAJOR`: The current Milo complete version as a string.
- `MILO_VERSION_MAJOR`: The current Milo major version.
- `MILO_VERSION_MINOR`: The current Milo minor version.
- `MILO_VERSION_PATCH` The current Milo patch version.
- `DEBUG`: If the debug informations are enabled or not.
- `NO_COPY`: If the `no-copy` mode is enabled.
- `AUTODETECT`: Set the parser to autodetect if the next message is a request or a response.
- `REQUEST`: Set the parser to only parse requests.
- `RESPONSE`: Set the parser to only parse response.
- `CONNECTION_*`: A `Connection` header value.
- `ERROR_*`: An error code.
- `METHOD_*`: An HTTP/RTSP request method.
- `STATE_*`: A parser state.

## `milo::Flags`

A struct representing the current compile flags of Milo:

- `debug`: If the debug informations are enabled or not.

## `milo::Callback`

All callback in Milo have the following signature (`Callback`):

```cpp
intptr_t(*)(milo::Parser*, const unsigned char*, uintptr_t)
```

where the parameters have the following meaning:

1. The current parser.
2. A data slice. Can be `NULL` (and in that case the next parameter will be 0).
3. The data length. Can be `0` (and in that case the previous parameter will be `NULL`).

If you are using Milo in `no-copy` mode (see above) then the callbacks signature changes as follows:

```rust
intptr_t(*)(milo::Parser*, uintptr_t, uintptr_t)
```

where the parameters have the following meaning:

1. The current parser.
2. The offset (relative to the last data passed to `milo::milo_parse`) where the current payload starts.
3. The data length. Can be `0` (and in that case the previous parameter will be `0`).

In both cases the return value must be `0` in case of success, any other value will halt the parser in error state.

## `milo::Callbacks`

A struct representing the callbacks for a parser. Here's the list of supported callbacks:

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

If you want to remove a previously set callback, you can use `milo::milo_noop`.

## `milo::Parser`

A struct representing a parser. It has the following fields:

- `owner` (`void*`): The owner of this parser. Use is reserved to the developer.
- `state` (`u8`): The current parser state.
- `position` (`uint32_t`): The current parser position in the slice in the current execution of `milo::milo_parse`.
- `parsed` (`u64`): The total bytes consumed from this parser.
- `paused` (`bool`): If the parser is paused.
- `error_code` (`u8`): The parser error. By default is `ERROR_NONE`.
- `error_description` (`const unsigned char*`): The parser error description.
- `error_description_len` (`uintptr_t`): The parser error description length.
- `unconsumed` (`const unsigned char*`): The unconsumed data from the previous execution of `milo::milo_parse`. _This is not available in `no-copy` mode (see above)._
- `unconsumed_len` (`uintptr_t`): The unconsumed data length from the previous execution of `milo::milo_parse`. _This is not available in `no-copy` mode (see above)._
- `id` (`uint8_t`): The current parser ID. Use is reserved to the developer.
- `mode` (`uint8_t`): The current parser mode. Can be `milo::AUTODETECT`, `milo::REQUEST` or `milo::RESPONSE`,
- `continue_without_data` (`bool`): If the next execution of the parse loop should execute even if there is no more data.
- `message_type` (`uint8_t`): The current message type. Can be `milo::REQUEST` or `milo::RESPONSE`.
- `is_connect` (`bool`): If the current request used `CONNECT` method.
- `method` (`uint8_t`): The current request method as integer.
- `status` (`uint32_t`): The current response status.
- `version_major` (`uint8_t`): The current message HTTP version major version.
- `version_minor` (`uint8_t`): The current message HTTP version minor version.
- `connection` (`uint8_t`): The value for the connection header. Can be `milo::CONNECTION_CLOSE`, `milo::CONNECTION_UPGRADE` or `milo::CONNECTION_KEEPALIVE` (which is the default when no header is set).
- `has_content_length` (`bool`): If the current request has a `Content-Length` header.
- `has_chunked_transfer_encoding` (`bool`): If the current request has a `Transfer-Encoding` header.
- `has_upgrade` (`bool`): If the current request has a `Connection: upgrade` header.
- `has_trailers` (`bool`): If the current request has a `Trailers` header.
- `content_length` (`uint64_t`): The value of the `Content-Length` header.
- `chunk_size` (`uint64_t`): The expected length of the next chunk.
- `remaining_content_length` (`uint64_t`): The missing data length of the body according to the `content_length` field.
- `remaining_chunk_size` (`uint64_t`): The missing data length of the next chunk according to the `chunk_size` field.
- `skip_body` (`bool`): If the parser should skip the body.
- `callbacks` (`Callbacks`): The callbacks for the current parser.

Most of the fields **MUST** be considered readonly. The only exception are:

- `owner`
- `id`
- `mode`
- `is_connect`
- `skip_body`
- `callbacks`

The only persisted fields across resets are `owner`, `id`, `mode` and `continue_without_data`.

## `milo::MessageTypes`

An enum listing all possible message types.

## `milo::Connections`

An enum listing all possible connection (`Connection` header value) types.

## `milo::Methods`

An enum listing all possible HTTP/RTSP methods.

## `milo::States`

An enum listing all possible parser states.

## `milo::Errors`

An enum listing all possible parser errors.

### `Flags milo::milo_flags()`

Returns a struct representing the current compile flags of Milo.

## `intptr_t milo_noop(const Parser *_parser, const unsigned char *_data, uintptr_t _len)`

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

## `void milo_reset(const Parser *parser, bool keep_parsed)`

Resets a parser. The second parameters specifies if to also reset the parsed counter.

## `uintptr_t milo_parse(const Parser *parser, const unsigned char *data, uintptr_t limit)`

Parses `data` up to `limit` characters.

It returns the number of consumed characters.

## `void milo_pause(const Parser *parser)`

Pauses the parser. The parser will have to be resumed via `milo::milo_resume`.

## `void milo_resume(const Parser *parser)`

Resumes the parser.

## `void milo_finish(const Parser *parser)`

Marks the parser as finished. Any new invocation of `milo::milo_parse` will put the parser in the error state.

## `const unsigned char *milo_state_string(const Parser *parser)`

Returns the current parser's state as string.

**The returned value MUST be freed using `milo::milo_free_string`.**

## `const unsigned char *milo_error_code_string(const Parser *parser)`

Returns the current parser's error state as string.

**The returned value MUST be freed using `milo::milo_free_string`.**

## `const unsigned char *milo_error_description_string(const Parser *parser)`

Returns the current parser's error descrition.

**The returned value MUST be freed using `milo::milo_free_string`.**
