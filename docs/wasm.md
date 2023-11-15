# Using Milo in JavaScript vis WebAssembly

Milo release is composed by the following files:

- `milo.d.ts`
- `milo.js`
- `milo_bg.wasm`
- `milo_bg.wasm.d.ts`
- `package.json`

All you need is to import the `milo.js` file (using `require` or `import`).

When downloading Milo, you can choose between 4 different flavors:

1. `release/with-copy`: Release version with copying enabled.
2. `release/no-copy`: Release version with copying disabled.
3. `debug/with-copy`: Debug version with copying enabled.
4. `debug/no-copy`: Debug version with copying disabled.

## `with-copy` vs `no-copy` mode

The `with-copy` mode of Milo buffers unconsumed data from parsing and automatically prepends it in the next invocation of the `parse` method.

The `no-copy` mode has the behavior above disabled.

# WebAssembly API

The module exports several constants (`*` is used to denote a family prefix):

- `AUTODETECT`: Set the parser to autodetect if the next message is a request or a response.
- `REQUEST`: Set the parser to only parse requests.
- `RESPONSE`: Set the parser to only parse response.
- `CONNECTION_*`: A `Connection` header value.
- `ERROR_*`: An error code.
- `METHOD_*`: An HTTP/RTSP request method.
- `STATE_*`: A parser state.

## `Parser`

A struct representing a parser. It has the following fields:

- `state` (`readonly u8`): The current parser state.
- `position` (`readonly bigint`): The current parser position.
- `paused` (`readonly bool`): If the parser is paused.
- `errorCode` (`readonly number`): The parser error. By default is `ERROR_NONE`.
- `errorString` (`readonly string`): The parser error as string. By default is `NONE`.
- `errorDescription` (`readonly string`): The parser error description.
- `id` (`number`): The current parser ID. Use is reserved to the developer.
- `mode` (`number`): The current parser mode. Can be `milo::AUTODETECT`, `milo::REQUEST` or `milo::RESPONSE`,
- `continueWithoutData` (`readonly bool`): If the next execution of the parse loop should execute even if there is no more data.
- `messageType` (`readonly number`): The current message type. Can be `milo::REQUEST` or `milo::RESPONSE`.
- `isConnect` (`bool`): If the current request used `CONNECT` method.
- `method` (`readonly number`): The current request method as integer.
- `status` (`readonly number`): The current response status.
- `versionMajor` (`readonly number`): The current message HTTP version major version.
- `versionMinor` (`readonly number`): The current message HTTP version minor version.
- `connection` (`readonly number`): The value for the connection header. Can be `milo::CONNECTION_CLOSE`, `milo::CONNECTION_UPGRADE` or `milo::CONNECTION_KEEPALIVE` (which is the default when no header is set).
- `hasContentLength` (`readonly bool`): If the current request has a `Content-Length` header.
- `hasChunkedTransferEncoding` (`readonly bool`): If the current request has a `Transfer-Encoding` header.
- `hasUpgrade` (`readonly bool`): If the current request has a `Connection: upgrade` header.
- `hasTrailers` (`readonly bool`): If the current request has a `Trailers` header.
- `contentLength` (`readonly bigint`): The value of the `Content-Length` header.
- `chunkSize` (`readonly bigint`): The expected length of the next chunk.
- `remainingContentLength` (`readonly bigint`): The missing data length of the body according to the `content_length` field.
- `remainingChunkSize` (`readonly bigint`): The missing data length of the next chunk according to the `chunk_size` field.
- `skipBody` (`bool`): If the parser should skip the body.

The parser supports the following callbacks setter:

- `setBeforeStateChange`: Invoked before the parser change its state. _Only invoked in debug mode_.
- `setAfterStateChange`: Invoked after the parser change its state. _Only invoked in debug mode_.
- `setOnError`: Invoked after the parsing fails.
- `setOnFinish`: Invoked after the parser is marked as finished.
- `setOnMessageStart`: Invoked after a new message starts.
- `setOnMessageComplete`: Invoked after a message finishes.
- `setOnRequest`: Invoked after the message is identified as a request.
- `setOnResponse`: Invoked after the message is identified as a response.
- `setOnReset`: Invoked after the parser is reset (either manually or after parsing a new message except the first oneset).
- `setOnMethod`: Invoked after the HTTP method has been parsed.
- `setOnUrl`: Invoked after the request URL has been parsed.
- `setOnProtocol`: Invoked after the request or response protocol has been parsed.
- `setOnVersion`: Invoked after the request or response version has been parsed.
- `setOnStatus`: Invoked after the response status has been parsed.
- `setOnReason`: Invoked after the response status reason has been parsed.
- `setOnHeaderName`: Invoked after a new header name has been parsed.
- `setOnHeaderValue`: Invoked after a new header value has been parsed.
- `setOnHeaders`: Invoked after headers are completed.
- `setOnConnect`: Invoked in `CONNECT` requests after headers have been completed.
- `setOnUpgrade`: Invoked after a connection is upgraded via a `Connection: upgrade` request header.
- `setOnChunkLength`: Invoked after a new chunk length has been parsed.
- `setOnChunkExtensionName`: Invoked after a new chunk extension name has been parsed.
- `setOnChunkExtensionValue`: Invoked after a new chunk extension value has been parsed.
- `setOnChunk`: Invoked after new chunk data is received.
- `setOnData`: Invoked after new body data is received (either chunked or not).
- `setOnBody`: Invoked after the body has been parsed. Note that this has no data attached so `on_data` must be used to setSave the body.
- `setOnTrailerName`: Invoked after a new trailer name has been parsed.
- `setOnTrailerValue`: Invoked after a new trailer value has been parsed.
- `setOnTrailers`: Invoked after trailers are completed.

Each callback can be invoked with either no arguments or the following arguments:

1. A data slice. (`Uint8Array`).
2. The data length.

If you are using Milo in `no-copy` mode (see above) then the callbacks arguments (when present) change as follows:

1. The offset (relative to the last data passed to `Parser.parser`) where the current payload starts.
2. The data length.

In both cases the return value must be `0` in case of success, any other number will halt the parser in error state.

Not returning an number will throw an error.

## `flags()`

Returns an object representing the current compile flags of Milo:

- `debug`: If the debug informations are enabled or not.
- `all_callbacks`: If Milo will invoke all headers callbacks.

## `MessageTypes`

An enum listing all possible message types.

## `Connections`

An enum listing all possible connection (`Connection` header value) types.

## `Methods`

An enum listing all possible HTTP/RTSP methods.

## `States`

An enum listing all possible parser states.

## `Errors`

An enum listing all possible parser errors.
