# Rust API

## `Callback`

All callbacks in Milo have the following signature (`Callback`):

```rust
type Callback = fn (&mut Parser, usize, usize)
```

where the parameters have the following meaning:

1. The current parser.
2. The payload offset. Can be `0`.
3. The data length. Can be `0`.

If length is `0`, it means the callback has no payload associated.

Callbacks are dispatched only when the corresponding `CALLBACK_ACTIVE_*` flag is set in the parser `active_callbacks` field.

Callbacks are disabled by default.

## Constants

The crate exports several constants (`*` is used to denote a family prefix):

- `ERROR_*`: An error code.
- `METHOD_*`: An HTTP request method.
- `CALLBACK_*`: A parser callback.
- `CALLBACK_ACTIVE_*`: A callback activation flag.
- `EVENT_*`: A parser event type.
- `EVENT_ACTIVE_*`: An event activation flag.
- `STATE_*`: A parser state.

Internal generated lookup tables used by the parser are not public API.

## Enums

All the enums below implement `TryFrom<u8>` and `Into<&str>` traits and have the `as_str` method.

### `Errors`

An enum listing all possible parser errors.

### `Methods`

An enum listing all possible HTTP methods recognized by Milo.

### `Callbacks`

An enum listing all possible parser callbacks.

### `Events`

An enum listing all possible parser events.

### `States`

An enum listing all possible parser states.

## Types

### `ParserCallbacks`

A struct representing the callbacks for a parser.

Here's the list of supported callbacks:

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

If you want to remove a previously set callback, you can use the `milo_noop` function also exported by this crate.

### `Parser`

A struct representing a parser. It has the following fields:

- `autodetect` (`bool`): If the parser should autodetect requests and responses. Enabled by default.
- `is_request` (`bool`): The configured or detected message type. Set this when `autodetect` is `false`.
- `paused` (`bool`): If the parser is paused.
- `manage_unconsumed` (`bool`): If the parser should automatically copy and prepend unconsumed data.
- `suspend_after_headers` (`bool`): If parsing should stop after headers have completed. Disabled by default.
- `continue_without_data` (`bool`): If the next execution of the parse loop should execute even if there is no more data.
- `is_connect` (`bool`): If the current request used `CONNECT` method.
- `skip_body` (`bool`): If the parser should skip the body.
- `debug` (`bool`): If debug tracing is enabled for this parser. It only affects tracing in debug-enabled builds.
- `max_start_line_length` (`usize`): Maximum allowed request/status line length. By default is `8192`.
- `max_header_length` (`usize`): Maximum allowed header length. By default is `8192`.
- `max_body_payload` (`u64`): Maximum body payload bytes consumed in a single `parse()` call. `0` means unlimited and is the default.
- `context` (`*mut c_void`): The context of this parser. Use is reserved to the developer.
- `state` (`u8`): The current parser state.
- `position` (`usize`): The current parser position in the slice in the current execution of `milo_parse`.
- `parsed` (`u64`): The total bytes consumed from this parser.
- `error_code` (`u8`): The parser error. By default is `ERROR_NONE`.
- `method` (`u8`): The current request method.
- `status` (`u32`): The current response status.
- `content_length` (`u64`): The value of the `Content-Length` header.
- `chunk_size` (`u64`): The expected length of the next chunk.
- `remaining_content_length` (`u64`): The missing data length of the body according to the `content_length` field.
- `remaining_chunk_size` (`u64`): The missing data length of the next chunk according to the `chunk_size` field.
- `has_content_length` (`bool`): If the current message has a `Content-Length` header.
- `has_transfer_encoding` (`bool`): If the current message has a `Transfer-Encoding` header.
- `has_chunked_transfer_encoding` (`bool`): If the current message is using chunked encoding.
- `has_connection_close` (`bool`): If the current message has a `Connection: close` token.
- `has_connection_upgrade` (`bool`): If the current message has a `Connection: upgrade` token.
- `has_upgrade` (`bool`): If the current message has an `Upgrade` header.
- `has_trailers` (`bool`): If the current message has a `Trailer` header.
- `active_callbacks` (`u64`): Active callback bitmask. Set to one or more `CALLBACK_ACTIVE_*` flags.
- `active_events` (`u64`): Active event bitmask. Set to one or more `EVENT_ACTIVE_*` flags.
- `callbacks` (`ParserCallbacks`): The callbacks for the current parser.
- `error_description` (`[u8; 255]`): The parser error description buffer. It is always NIL-terminated.
- `error_description_len` (`u8`): The parser error description length, excluding the NIL terminator. Error descriptions are clamped to 254 bytes.
- `unconsumed` (`*const c_uchar`): The unconsumed data from the previous execution of `parse` when `manage_unconsumed` is `true`.
- `unconsumed_len` (`usize`): The unconsumed data length from the previous execution of `parse` when `manage_unconsumed` is `true`.
- `events` (`*mut c_uchar`): Parser-owned event buffer.

All the fields **MUST** be considered readonly, with the following exceptions:

- `autodetect`
- `is_request`
- `manage_unconsumed`
- `suspend_after_headers`
- `continue_without_data`
- `is_connect`
- `skip_body`
- `debug`
- `max_start_line_length`
- `max_header_length`
- `max_body_payload`
- `context`
- `active_callbacks`
- `active_events`
- `callbacks`

## Events

Events are parser-owned records written to `Parser::events` during parsing. They are disabled by default. Enable them by setting `Parser::active_events` to one or more `EVENT_ACTIVE_*` flags.

Callbacks are replayed from the same event buffer. Setting `active_callbacks` also enables event emission for those callbacks, then callbacks are invoked in event order before `parse()` returns.

The event buffer is terminated by `EVENT_END`. Do not rely on the internal buffer size; always stop reading at `EVENT_END`. Event payload integers are little-endian and may be unaligned, so read multi-byte values with unaligned reads.

If an active event would exceed the internal event buffer, parsing stops before consuming the data that would have produced the event. This is not a parser error and does not pause the parser. Call `parse()` again after draining the event buffer.

## Body Payload Limit

`max_body_payload` limits how many body payload bytes a single `parse()` invocation can consume. The default value is `0`, which means unlimited.

When the limit is reached, `parse()` returns normally with `consumed < limit` and leaves the remaining input unconsumed. This is not a parser error and does not pause the parser. The next `parse()` invocation continues from the same parser state.

The limit applies only to body payload bytes. Framing bytes such as chunk headers, chunk CRLFs, and trailers are not counted.

## Suspend After Headers

`suspend_after_headers` stops parsing after the final header terminator has been consumed and `on_headers` has been emitted. `parse()` returns normally, the parser is not paused, and the next `parse()` call continues with body decision and body parsing.

### Range events

Most events use this payload:

```text
u8  type
u32 at
u32 len
```

`type` is one of the `EVENT_*` constants. `at` and `len` are relative to the last input passed to `parse()`. `len` can be `0`.

`EVENT_STATE_CHANGE` is debug-only and uses the same payload. For this event, `len` contains the new parser state id as a `u32`. Callback replay passes that value as the callback `size` argument.

### Metadata events

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

### Error events

`EVENT_ERROR` uses this payload:

```text
u8  type
u32 at
u8  error_code
```

### Reading events

```rust
use core::ptr;

use milo_parser::{EVENT_DATA, EVENT_END, Parser};

fn drain(parser: &Parser) {
  let mut cursor = 0usize;

  loop {
    let event_type = unsafe { *parser.events.add(cursor) };

    match event_type {
      EVENT_END => break,
      EVENT_DATA => {
        let at = u32::from_le(unsafe { ptr::read_unaligned(parser.events.add(cursor + 1) as *const u32) }) as usize;
        let len = u32::from_le(unsafe { ptr::read_unaligned(parser.events.add(cursor + 5) as *const u32) }) as usize;
        // Use at and len.
        cursor += 9;
      }
      _ => {
        // Decode other events according to their payload type.
        break;
      }
    }
  }
}
```

#### `Parser::new() -> Parser`

Creates a new parser.

#### `Parser::parse(&mut self, data: *const c_uchar, limit: usize) -> usize`

Parses `data` up to `limit` characters.

It returns the number of consumed characters.

#### `Parser::reset(&mut self, keep_parsed: bool)`

Resets a parser. The second parameters specifies if to also reset the
parsed counter.

The following fields are not modified:

- `position`
- `context`
- `autodetect`
- `is_request`
- `manage_unconsumed`
- `suspend_after_headers`
- `continue_without_data`
- `debug`
- `max_start_line_length`
- `max_header_length`
- `context`
- `active_callbacks`
- `callbacks`

#### `Parser::clear(&mut self)`

Clears all values about the message in the parser.

The `autodetect` and `is_request` fields are not cleared.

#### `Parser::pause(&mut self)`

Pauses the parser. The parser will have to be resumed via `Parser::resume`.

#### `Parser::resume(&mut self)`

Resumes the parser.

#### `Parser::complete(&mut self)`

Completes the current message without consuming more input.

This emits normal completion events and performs the same completion transition
used by `parse`. It is valid only while the parser is in `BODY_DECISION`,
`TUNNEL`, `BODY_VIA_CONTENT_LENGTH`, `BODY_WITH_NO_LENGTH`, `CHUNK_HEADER`, or
`TRAILER`. Other states fail with `ERROR_UNEXPECTED_STATE`.

#### `Parser::finish(&mut self)`

Marks the parser as finished. Any new data received via `parse` will
put the parser in the error state.

#### `Parser::fail(&mut self, code: usize, description: &str)`

Marks the parsing a failed, setting a error code and and error message.

It always returns zero for internal use.

#### `Parser::state_str(&self) -> &str`

Returns the current parser's state as string.

#### `Parser::error_code_str(&self) -> &str`

Returns the current parser's error state as string.

#### `Parser::error_description_str(&self) -> &str`

Returns the current parser's error description.

## Methods

### `milo_has_debug() -> bool`

Returns `true` if debug informations are available in this build.

### `milo_noop(_parser: &Parser, _data: *const c_uchar, _len: usize)`

A callback that simply returns `0`.

Use this callback as pointer when you want to remove a callback from the parser.

## FFI public interface

The following functions are defined to allow Rust to work in a C++ or WebAssembly environment.

While you can use these functions within Rust, it makes little sense as they only call the corresponding method of the parser passed as first argument.

### `CStringWithLength`

A struct representing a string containing the following fields:

- `ptr` (`*const c_uchar`): The string data pointer.
- `len` (`usize`): The string length.

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
- `autodetect`
- `is_request`
- `manage_unconsumed`
- `continue_without_data`
- `context`

### `milo_clear(parser: *mut Parser)`

Clears all values about the message in the parser.

The `autodetect` and `is_request` fields are not cleared.

### `milo_pause(parser: *mut Parser)`

Pauses the parser. The parser will have to be resumed via `milo_parser::milo_resume`.

### `milo_resume(parser: *mut Parser)`

Resumes the parser.

### `milo_complete(parser: *mut Parser)`

Completes the current message without consuming more input.

This emits normal completion events and performs the same completion transition
used by `milo_parse`. It is valid only while the parser is in `BODY_DECISION`,
`TUNNEL`, `BODY_VIA_CONTENT_LENGTH`, `BODY_WITH_NO_LENGTH`, `CHUNK_HEADER`, or
`TRAILER`. Other states fail with `ERROR_UNEXPECTED_STATE`.

### `milo_finish(parser: *mut Parser)`

Marks the parser as finished. Any new invocation of `milo_parse` will put the parser in the error state.

### `milo_fail(parser: *mut Parser, code: usize, description: CStringWithLength)`

Marks the parsing a failed, setting a error code and and error message.

### `milo_state_string(parser: *mut Parser) -> *const c_uchar`

Returns the current parser's state as string.

**The returned value MUST be freed using `milo_free_string`.**

### `milo_method_to_string(method: u8) -> *const c_uchar`

Returns a parser method as string.

**The returned value MUST be freed using `milo_free_string`.**

### `milo_error_to_string(error: u8) -> *const c_uchar`

Returns a parser error as string.

**The returned value MUST be freed using `milo_free_string`.**

### `milo_callback_to_string(callback: u8) -> *const c_uchar`

Returns a parser callback as string.

**The returned value MUST be freed using `milo_free_string`.**

### `milo_state_to_string(state: u8) -> *const c_uchar`

Returns a parser state as string.

**The returned value MUST be freed using `milo_free_string`.**

### `milo_event_to_string(event: u8) -> *const c_uchar`

Returns a parser event as string. `EVENT_END` returns `END`.

**The returned value MUST be freed using `milo_free_string`.**

### `milo_error_code_string(parser: *mut Parser) -> *const c_uchar`

Returns the current parser's error state as string.

**The returned value MUST be freed using `milo_free_string`.**

### `milo_error_description_string(parser: *mut Parser) -> *const c_uchar`

Returns the current parser's error description.

**The returned value MUST be freed using `milo_free_string`.**
