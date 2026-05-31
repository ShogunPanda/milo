# C++ API

## Linking Milo in C++

Milo release is composed by a header file `milo.h` and static library `libmilo.a`

The exact command to build milo is dependent on your compiler. For instance, for `clang` the command is the following:

```bash
clang++ -std=c++11 -I $MILO_DIR -o output main.cc $MILO_DIR/libmilo.a
```

where `$MILO_DIR` is the directory containing the `milo.h` and `libmilo.a` files.

When downloading Milo, you can choose between `debug` or `release` version:

## Callbacks handling

All callbacks in Milo have the following signature (`Callback`):

```cpp
void(*)(milo_parser::Parser*, uintptr_t, uintptr_t)
```

where the parameters have the following meaning:

1. The current parser.
2. The payload offset. Can be `0`.
3. The data length. Can be `0`.

If length is `0`, it means the callback has no payload associated.

Callbacks are dispatched only when the corresponding `CALLBACK_ACTIVE_*` flag is set in the parser `active_callbacks` field.

Callbacks are disabled by default.

## Constants

The file `milo.h` defines several constants (`*` is used to denote a family prefix):

- `MILO_VERSION`: The current Milo complete version as a string.
- `MILO_VERSION_MAJOR`: The current Milo major version.
- `MILO_VERSION_MINOR`: The current Milo minor version.
- `MILO_VERSION_PATCH` The current Milo patch version.
- `DEBUG`: If debug information is enabled or not.
- `ERROR_*`: An error code.
- `METHOD_*`: An HTTP request method.
- `CALLBACK_*`: A parser callback.
- `CALLBACK_ACTIVE_*`: A callback activation flag.
- `STATE_*`: A parser state.

Internal generated lookup tables used by the parser are not exported in `milo.h`.

## Types

### `milo_parser::CStringWithLength`

A struct representing a string containing the following fields:

- `ptr` (`const unsigned char *`): The string data pointer.
- `len` (`uintptr_t`): The string length.

### `milo_parser::ParserCallbacks`

A struct representing the callbacks for a parser. Here's the list of supported callbacks:

- `on_state_change`: Invoked after the parser changes its state. _Only invoked in debug mode_.
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
- `on_upgrade`: Invoked after a request or response enters tunnel mode via `Upgrade` and `Connection: upgrade`.
- `on_chunk_length`: Invoked after a new chunk length has been parsed.
- `on_chunk_extension_name`: Invoked after a new chunk extension name has been parsed.
- `on_chunk_extension_value`: Invoked after a new chunk extension value has been parsed.
- `on_chunk`: Invoked after new chunk data is received.
- `on_data`: Invoked after new body data is received (either chunked or not).
- `on_body`: Invoked after the body has been parsed. Note that this has no data attached so `on_data` must be used to save the body.
- `on_trailer_name`: Invoked after a new trailer name has been parsed.
- `on_trailer_value`: Invoked after a new trailer value has been parsed.
- `on_trailers`: Invoked after trailers are completed.

If you want to remove a previously set callback, you can use `milo_parser::milo_noop`.

### `milo_parser::Parser`

A struct representing a parser. It has the following fields:

- `autodetect` (`bool`): If the parser should autodetect requests and responses. Enabled by default.
- `is_request` (`bool`): The configured or detected message type. Set this when `autodetect` is `false`.
- `paused` (`bool`): If the parser is paused.
- `manage_unconsumed` (`bool`): If the parser should automatically copy and prepend unconsumed data.
- `continue_without_data` (`bool`): If the next execution of the parse loop should execute even if there is no more data.
- `is_connect` (`bool`): If the current request used `CONNECT` method.
- `skip_body` (`bool`): If the parser should skip the body.
- `max_start_line_length` (`uintptr_t`): Maximum allowed request/status line length. By default is `8192`.
- `max_header_length` (`uintptr_t`): Maximum allowed header length. By default is `8192`.
- `context` (`void*`): The context of this parser. Use is reserved to the developer.
- `state` (`uint8_t`): The current parser state.
- `position` (`uintptr_t`): The current parser position in the slice in the current execution of `milo_parse`.
- `parsed` (`uint64_t`): The total bytes consumed from this parser.
- `error_code` (`uint8_t`): The parser error. By default is `ERROR_NONE`.
- `method` (`uint8_t`): The current request method.
- `status` (`uint32_t`): The current response status.
- `version_major` (`uint8_t`): The current message HTTP version major version.
- `version_minor` (`uint8_t`): The current message HTTP version minor version.
- `content_length` (`uint64_t`): The value of the `Content-Length` header.
- `chunk_size` (`uint64_t`): The expected length of the next chunk.
- `remaining_content_length` (`uint64_t`): The missing data length of the body according to the `content_length` field.
- `remaining_chunk_size` (`uint64_t`): The missing data length of the next chunk according to the `chunk_size` field.
- `has_content_length` (`bool`): If the current message has a `Content-Length` header.
- `has_transfer_encoding` (`bool`): If the current message has a `Transfer-Encoding` header.
- `has_chunked_transfer_encoding` (`bool`): If the current message is using chunked encoding.
- `has_connection_close` (`bool`): If the current message has a `Connection: close` token.
- `has_connection_upgrade` (`bool`): If the current message has a `Connection: upgrade` token.
- `has_upgrade` (`bool`): If the current message has an `Upgrade` header.
- `has_trailers` (`bool`): If the current message has a `Trailer` header.
- `active_callbacks` (`uint64_t`): Active callback bitmask. Set to one or more `CALLBACK_ACTIVE_*` values.
- `callbacks` (`ParserCallbacks`): The callbacks for the current parser.
- `error_description` (`const unsigned char*`): The parser error description.
- `error_description_len` (`uint16_t`): The parser error description length.
- `unconsumed` (`const unsigned char*`): The unconsumed data from the previous execution of `parse` when `manage_unconsumed` is `true`.
- `unconsumed_len` (`uintptr_t`): The unconsumed data length from the previous execution of `parse` when `manage_unconsumed` is `true`.

All the fields **MUST** be considered readonly, with the following exceptions:

- `autodetect`
- `is_request`
- `manage_unconsumed`
- `continue_without_data`
- `is_connect`
- `skip_body`
- `max_start_line_length`
- `max_header_length`
- `context`
- `active_callbacks`
- `callbacks`

## Enumerations

### `milo_parser::Errors`

An enum listing all possible parser errors.

### `milo_parser::Methods`

An enum listing all possible HTTP methods recognized by Milo.

### `milo_parser::Callbacks`

An enum listing all possible parser callbacks.

### `milo_parser::States`

An enum listing all possible parser states.

## Methods

### `void milo_noop(Parser *_parser, uintptr_t _at, uintptr_t _len)`

A callback that does nothing.

Use this callback as pointer when you want to remove a callback from the parser.

### `void milo_free_string(CStringWithLength s)`

Release memory from a string previously obtained from other APIs.

**By convention, all milo's C++ function which ends in `_string` MUST have their value freed up with this function when done.**

### `Parser *milo_create()`

Creates a new parser.

**The returned value MUST be destroyed later using `milo_destroy`.**

### `void milo_destroy(Parser *ptr)`

Destroys a parser.

### `uintptr_t milo_parse(Parser *parser, const unsigned char *data, uintptr_t limit)`

Parses `data` up to `limit` characters.

It returns the number of consumed characters.

### `void milo_reset(Parser *parser, bool keep_parsed)`

Resets a parser. The second parameters specifies if to also reset the
parsed counter.

The following fields are not modified:

- `position`
- `context`
- `autodetect`
- `is_request`
- `manage_unconsumed`
- `continue_without_data`
- `max_start_line_length`
- `max_header_length`
- `context`
- `active_callbacks`
- `callbacks`

### `void milo_clear(Parser *parser)`

Clears all values about the message in the parser.

The `autodetect` and `is_request` fields are not cleared.

### `void milo_pause(Parser *parser)`

Pauses the parser. The parser will have to be resumed via `milo_parser::milo_resume`.

### `void milo_resume(Parser *parser)`

Resumes the parser.

### `void milo_finish(Parser *parser)`

Marks the parser as finished. Any new invocation of `milo_parser::milo_parse` will put the parser in the error state.

### `void milo_fail(Parser *parser, uintptr_t code, CStringWithLength description)`

Marks the parsing a failed, setting a error code and and error message.

### `CStringWithLength *milo_state_string(Parser *parser)`

Returns the current parser's state as string.

**The returned value MUST be freed using `milo_parser::milo_free_string`.**

### `CStringWithLength *milo_error_code_string(Parser *parser)`

Returns the current parser's error state as string.

**The returned value MUST be freed using `milo_parser::milo_free_string`.**

### `CStringWithLength *milo_error_description_string(Parser *parser)`

Returns the current parser's error description.

**The returned value MUST be freed using `milo_parser::milo_free_string`.**
