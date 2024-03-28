# Linking Milo in C++

Milo release is composed by a header file `milo.h` and static library `libmilo.a`

The exact command to build milo is dependent on your compiler. For instance, for `clang` the command is the following:

```bash
clang++ -std=c++11 -I $MILO_DIR -o output $MILO_DIR/libmilo.a main.cc
```

where `$MILO_DIR` is the directory containing the `milo.h` and `libmilo.a` files.

When downloading Milo, you can choose between `debug` or `release` version:

# C++ API

The file `milo.h` defines several constants (`*` is used to denote a family prefix):

- `MILO_VERSION_MAJOR`: The current Milo complete version as a string.
- `MILO_VERSION_MAJOR`: The current Milo major version.
- `MILO_VERSION_MINOR`: The current Milo minor version.
- `MILO_VERSION_PATCH` The current Milo patch version.
- `DEBUG`: If the debug informations are enabled or not.
- `MESSAGE_TYPE_*`: The type of the parser: it can autodetect (default) or only parse requests or response.
- `ERROR_*`: An error code.
- `METHOD_*`: An HTTP/RTSP request method.
- `CONNECTION_*`: A `Connection` header value.
- `CALLBACK_*`: A parser callback.
- `STATE_*`: A parser state.

## `milo::Flags`

A struct representing the current compile flags of Milo:

- `debug`: If the debug informations are enabled or not.

### `milo::CStringWithLength`

A struct representing a string containing the following fields:

- `ptr` (`const unsigned char *`): The string data pointer.
- `len` (`uintptr_t`): The string length.

## `milo::ParserCallbacks`

All callback in Milo have the following signature (`Callback`):

```cpp
void(*)(milo::Parser*, uintptr_t, uintptr_t)
```

where the parameters have the following meaning:

1. The current parser.
2. The payload offset. Can be `0`.
3. The data length. Can be `0`.

If both offset and length are `0`, it means the callback has no payload associated.

## `milo::ParserCallbacks`

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

- `mode` (`uintptr_t`): The current parser mode. Can be `MESSAGE_TYPE_AUTODETECT`, `MESSAGE_TYPE_REQUEST` or `MESSAGE_TYPE_RESPONSE`,
- `paused` (`bool`): If the parser is paused.
- `manage_unconsumed` (`bool`): If the parser should automatically copy and prepend unconsumed data.
- `continue_without_data` (`bool`): If the next execution of the parse loop should execute even if there is no more data.
- `is_connect` (`bool`): If the current request used `CONNECT` method.
- `skip_body` (`bool`): If the parser should skip the body.
- `owner` (`void*`): The context of this parser. Use is reserved to the developer.
- `state` (`uintptr_t`): The current parser state.
- `position` (`uintptr_t`): The current parser position in the slice in the current execution of `milo_parse`.
- `parsed` (`uint64_t`): The total bytes consumed from this parser.
- `error_code` (`uintptr_t`): The parser error. By default is `ERROR_NONE`.
- `message_type` (`uintptr_t`): The current message type. Can be `MESSAGE_TYPE_REQUEST` or `MESSAGE_TYPE_RESPONSE`.
- `method` (`uintptr_t`): The current request method.
- `status` (`uintptr_t`): The current response status.
- `version_major` (`uintptr_t`): The current message HTTP version major version.
- `version_minor` (`uintptr_t`): The current message HTTP version minor version.
- `connection` (`uintptr_t`): The value for the connection header. Can be `CONNECTION_CLOSE`, `CONNECTION_UPGRADE` or `CONNECTION_KEEPALIVE` (which is the default when no header is set).
- `content_length` (`uint64_t`): The value of the `Content-Length` header.
- `chunk_size` (`uint64_t`): The expected length of the next chunk.
- `remaining_content_length` (`uint64_t`): The missing data length of the body according to the `content_length` field.
- `remaining_chunk_size` (`uint64_t`): The missing data length of the next chunk according to the `chunk_size` field.
- `has_content_length` (`bool`): If the current message has a `Content-Length` header.
- `has_chunked_transfer_encoding` (`bool`): If the current message has a `Transfer-Encoding` header.
- `has_upgrade` (`bool`): If the current message has a `Connection: upgrade` header.
- `has_trailers` (`bool`): If the current message has a `Trailers` header.
- `callbacks` (`ParserCallbacks`): The callbacks for the current parser.
- `error_description` (`const unsigned char*`): The parser error description.
- `error_description_len` (`uintptr_t`): The parser error description length.
- `unconsumed` (`const unsigned char*`): The unconsumed data from the previous execution of `parse` when `manage_unconsumed` is `true`.
- `unconsumed_len` (`uintptr_t`): The unconsumed data length from the previous execution of `parse` when `manage_unconsumed` is `true`.

All the fields **MUST** be considered readonly, with the following exceptions:

- `mode`
- `manage_unconsumed`
- `continue_without_data`
- `is_connect`
- `skip_body`
- `context`
- `callbacks`

## `milo::MessageTypes`

An enum listing all possible message types.

## `milo::Errors`

An enum listing all possible parser errors.

## `milo::Methods`

An enum listing all possible HTTP/RTSP methods.

## `milo::Connections`

An enum listing all possible connection (`Connection` header value) types.

## `milo::Callbacks`

An enum listing all possible parser callbacks.

## `milo::States`

An enum listing all possible parser states.

### `Flags milo::milo_flags()`

Returns a struct representing the current compile flags of Milo.

## `void milo_noop(Parser *_parser, uintptr_t _at, uintptr_t _len)`

A callback that does nothing.

Use this callback as pointer when you want to remove a callback from the parser.

## `void milo_free_string(CStringWithLength s)`

Release memory from a string previously obtained from other APIs.

**By convention, all milo's C++ function which ends in `_string` MUST have their value freed up with this function when done.**

## `Parser *milo_create()`

Creates a new parser.

**The returned value MUST be destroyed later using `milo_destroy`.**

## `void milo_destroy(Parser *ptr)`

Destroys a parser.

### `uintptr_t milo_parse(Parser *parser, const unsigned char *data, uintptr_t limit)`

Parses `data` up to `limit` characters.

It returns the number of consumed characters.

## `void milo_reset(Parser *parser, bool keep_parsed)`

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

## `void milo_pause(Parser *parser)`

Pauses the parser. The parser will have to be resumed via `milo::milo_resume`.

## `void milo_resume(Parser *parser)`

Resumes the parser.

## `void milo_finish(Parser *parser)`

Marks the parser as finished. Any new invocation of `milo::milo_parse` will put the parser in the error state.

### `milo_fail(Parser *parser, uintptr_t code, CStringWithLength description)`

Marks the parsing a failed, setting a error code and and error message.

## `CStringWithLength *milo_state_string(Parser *parser)`

Returns the current parser's state as string.

**The returned value MUST be freed using `milo::milo_free_string`.**

## `CStringWithLength *milo_error_code_string(Parser *parser)`

Returns the current parser's error state as string.

**The returned value MUST be freed using `milo::milo_free_string`.**

## `CStringWithLength *milo_error_description_string(Parser *parser)`

Returns the current parser's error descrition.

**The returned value MUST be freed using `milo::milo_free_string`.**
