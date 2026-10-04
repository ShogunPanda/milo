import assert from 'node:assert/strict'
import { it } from 'node:test'
import { createParser, setup } from './helpers.js'

function requestParser (t) {
  const context = createParser(t, setup)
  context.milo.setShouldAutodetect(context.parser, false)
  context.milo.setIsRequest(context.parser, true)
  return context
}

function responseParser (t) {
  const context = createParser(t, setup)
  context.milo.setShouldAutodetect(context.parser, false)
  context.milo.setIsRequest(context.parser, false)
  return context
}

function assertOk (milo, parser) {
  assert.notEqual(milo.getState(parser), milo.STATE_ERROR, milo.getErrorDescription(parser))
  assert.equal(milo.getErrorCode(parser), milo.ERROR_NONE)
}

function assertError (milo, parser) {
  assert.equal(milo.getState(parser), milo.STATE_ERROR)
}

// RFC token syntax allows `|` in header names.
it('compliance_header_name_allows_pipe', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nX|Y: z\r\nContent-Length: 0\r\n\r\n')
  assertOk(milo, parser)
})

// RFC token syntax rejects `,` in header names.
it('compliance_header_name_rejects_comma', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nX,Y: z\r\nContent-Length: 0\r\n\r\n')
  assertError(milo, parser)
})

// RFC token syntax allows `|` in trailer names when chunked framing is valid.
it('compliance_trailer_name_allows_pipe', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX|Y: z\r\n\r\n')
  assertOk(milo, parser)
})

// RFC token syntax rejects `,` in trailer names.
it('compliance_trailer_name_rejects_comma', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX,Y: z\r\n\r\n')
  assertError(milo, parser)
})

// Unknown valid method tokens are accepted as extension methods.
it('compliance_unknown_method_token_is_accepted', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('FOO|BAR / HTTP/1.1\r\n\r\n')
  assertOk(milo, parser)
  assert.equal(milo.getMethod(parser), milo.METHOD_OTHER)
})

// Invalid unknown method tokens are rejected.
it('compliance_unknown_method_token_rejects_comma', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('BAD,METHOD / HTTP/1.1\r\n\r\n')
  assertError(milo, parser)
})

// PRI is only accepted with HTTP/2.0 for switch-over tunneling.
it('compliance_pri_requires_http2', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('PRI * HTTP/1.1\r\n\r\n')
  assertError(milo, parser)
})

// PRI with HTTP/2.0 enters tunnel mode instead of parsing HTTP/1.1 headers.
it('compliance_pri_http2_enters_tunnel', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n')
  assert.equal(milo.getState(parser), milo.STATE_TUNNEL)
})

// HTTP/2.0 is rejected for normal requests.
it('compliance_http2_request_rejected', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('GET / HTTP/2.0\r\n\r\n')
  assertError(milo, parser)
})

// HTTP/2.0 is rejected for responses.
it('compliance_http2_response_rejected', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/2.0 200 OK\r\n\r\n')
  assertError(milo, parser)
})

// PRI with HTTP/2.0 must be followed by the exact HTTP/2 connection preface suffix.
it('compliance_pri_http2_invalid_preface_rejected', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('PRI * HTTP/2.0\r\ngarbage\r\n\r\n')
  assertError(milo, parser)
})

// RTSP is not detected or accepted as an HTTP response protocol.
it('compliance_rtsp_response_rejected', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('RTSP/1.0 200 OK\r\n\r\n')
  assertError(milo, parser)
})

// Request targets cannot contain fragments.
it('compliance_request_target_rejects_fragment', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('GET /path#fragment HTTP/1.1\r\n\r\n')
  assertError(milo, parser)
})

// Milo intentionally rejects bodies on GET requests.
it('compliance_get_body_rejected', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('GET / HTTP/1.1\r\nContent-Length: 1\r\n\r\nx')
  assertError(milo, parser)
})

// Milo intentionally rejects bodies on HEAD requests.
it('compliance_head_body_rejected', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('HEAD / HTTP/1.1\r\nContent-Length: 1\r\n\r\nx')
  assertError(milo, parser)
})

// Methods other than GET and HEAD can carry valid body framing.
it('compliance_post_body_accepted', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('POST / HTTP/1.1\r\nContent-Length: 1\r\n\r\nx')
  assertOk(milo, parser)
})

// 205 responses complete after headers like other no-body statuses.
it('compliance_205_without_body_completes', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 205 Reset Content\r\n\r\n')
  assert.equal(milo.getState(parser), milo.STATE_START)
})

// 205 responses reject Content-Length as body framing.
it('compliance_205_content_length_rejected', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 205 Reset Content\r\nContent-Length: 0\r\n\r\n')
  assertError(milo, parser)
})

// 205 responses reject Transfer-Encoding as body framing.
it('compliance_205_transfer_encoding_rejected', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 205 Reset Content\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n')
  assertError(milo, parser)
})

// No-body responses except 304 reject Content-Length in strict Milo mode.
it('compliance_no_body_status_content_length_rejected', t => {
  for (const status of ['100 Continue', '204 No Content', '205 Reset Content']) {
    const { milo, parser, parse } = responseParser(t)
    parse(`HTTP/1.1 ${status}\r\nContent-Length: 0\r\n\r\n`)
    assertError(milo, parser)
  }
})

// 304 allows Content-Length as metadata but still has no body.
it('compliance_304_content_length_accepted_without_body', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 304 Not Modified\r\nContent-Length: 10\r\n\r\nHTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n')
  assertOk(milo, parser)
  assert.equal(milo.getState(parser), milo.STATE_START)
})

// No-body responses reject Transfer-Encoding in strict Milo mode.
it('compliance_no_body_status_transfer_encoding_rejected', t => {
  for (const status of ['100 Continue', '204 No Content', '304 Not Modified']) {
    const { milo, parser, parse } = responseParser(t)
    parse(`HTTP/1.1 ${status}\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n`)
    assertError(milo, parser)
  }
})

// Trailer is invalid without chunked transfer coding.
it('compliance_trailer_without_chunked_rejected', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('POST / HTTP/1.1\r\nTrailer: X\r\n\r\n')
  assertError(milo, parser)
})

// Upgrade does not bypass Trailer validation.
it('compliance_upgrade_trailer_without_chunked_rejected', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: websocket\r\nTrailer: X\r\n\r\n')
  assertError(milo, parser)
})

// Request upgrade may parse a chunked body and trailers before tunneling.
it('compliance_request_upgrade_chunked_trailers_before_tunnel', t => {
  const { milo, parser, parse } = requestParser(t)
  parse(
    'POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: websocket\r\nTransfer-Encoding: chunked\r\nTrailer: X\r\n\r\n0\r\nX: y\r\n\r\n'
  )
  assert.equal(milo.getState(parser), milo.STATE_TUNNEL)
})

// Response upgrade cannot use Trailer without valid chunked framing.
it('compliance_response_upgrade_trailer_rejected', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 101 Switching Protocols\r\nConnection: upgrade\r\nUpgrade: websocket\r\nTrailer: X\r\n\r\n')
  assertError(milo, parser)
})

// Valid unknown Connection options are accepted and ignored.
it('compliance_connection_unknown_token_accepted', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nConnection: foo\r\nContent-Length: 0\r\n\r\n')
  assertOk(milo, parser)
})

// Invalid unknown Connection options are rejected.
it('compliance_connection_unknown_token_rejected', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nConnection: foo@bar\r\nContent-Length: 0\r\n\r\n')
  assertError(milo, parser)
})

// Empty Connection list items are rejected.
it('compliance_connection_empty_item_rejected', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nConnection: close,,upgrade\r\nContent-Length: 0\r\n\r\n')
  assertError(milo, parser)
})

// Connection close finishes the parser and rejects subsequent data.
it('compliance_connection_close_rejects_later_data', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 0\r\n\r\nx')
  assertError(milo, parser)
})

// Upgrade values are comma-separated protocol tokens without special known values.
it('compliance_upgrade_tokens_accepted', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: foo, HTTP/2.0\r\n\r\n')
  assert.equal(milo.getState(parser), milo.STATE_TUNNEL)
})

// Upgrade protocol values reject empty protocol names.
it('compliance_upgrade_empty_protocol_name_rejected', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: /2.0\r\n\r\n')
  assertError(milo, parser)
})

// Upgrade protocol values reject empty protocol versions.
it('compliance_upgrade_empty_protocol_version_rejected', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: HTTP/\r\n\r\n')
  assertError(milo, parser)
})

// Upgrade protocol values reject more than one protocol version separator.
it('compliance_upgrade_extra_protocol_separator_rejected', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: HTTP/2/extra\r\n\r\n')
  assertError(milo, parser)
})

// Empty Upgrade values are rejected.
it('compliance_upgrade_empty_rejected', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: \r\n\r\n')
  assertError(milo, parser)
})

// Invalid Upgrade tokens are rejected.
it('compliance_upgrade_invalid_token_rejected', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: foo@bar\r\n\r\n')
  assertError(milo, parser)
})

// Empty Upgrade list items are rejected.
it('compliance_upgrade_empty_item_rejected', t => {
  const { milo, parser, parse } = requestParser(t)
  parse('POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: foo,,bar\r\n\r\n')
  assertError(milo, parser)
})

// Unquoted chunk extension values must be RFC tokens.
it('compliance_chunk_extension_unquoted_token_value', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo=bar|baz\r\nx\r\n0\r\n\r\n')
  assertOk(milo, parser)
})

// Unquoted chunk extension values reject spaces.
it('compliance_chunk_extension_unquoted_space_rejected', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo=bar baz\r\nx\r\n0\r\n\r\n')
  assertError(milo, parser)
})

// Unquoted chunk extension values reject non-token characters.
it('compliance_chunk_extension_unquoted_at_rejected', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo=bar@baz\r\nx\r\n0\r\n\r\n')
  assertError(milo, parser)
})

// Quoted chunk extension values may contain spaces.
it('compliance_chunk_extension_quoted_space_accepted', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo="bar baz"\r\nx\r\n0\r\n\r\n')
  assertOk(milo, parser)
})

// Quoted chunk extension values may contain quoted-pair escaped quotes.
it('compliance_chunk_extension_quoted_escaped_quote_accepted', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo="bar\\"baz"\r\nx\r\n0\r\n\r\n')
  assertOk(milo, parser)
})

// Quoted chunk extension values may contain quoted-pair escaped backslashes.
it('compliance_chunk_extension_quoted_escaped_backslash_accepted', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo="bar\\\\baz"\r\nx\r\n0\r\n\r\n')
  assertOk(milo, parser)
})

// Quoted chunk extension values may contain horizontal tabs.
it('compliance_chunk_extension_quoted_tab_accepted', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo="bar\tbaz"\r\nx\r\n0\r\n\r\n')
  assertOk(milo, parser)
})

// Quoted chunk extension values may contain obs-text.
it('compliance_chunk_extension_quoted_obs_text_accepted', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo="bar\u0080baz"\r\nx\r\n0\r\n\r\n')
  assertOk(milo, parser)
})

// Quoted chunk extension values reject bare control characters other than HTAB.
it('compliance_chunk_extension_quoted_control_rejected', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo="bar\u0001baz"\r\nx\r\n0\r\n\r\n')
  assertError(milo, parser)
})

// Quoted-pair in chunk extension values rejects escaped control characters.
it('compliance_chunk_extension_quoted_pair_control_rejected', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo="bar\\\u0001baz"\r\nx\r\n0\r\n\r\n')
  assertError(milo, parser)
})

// Bare CR is rejected in HTTP framing.
it('compliance_bare_cr_rejected', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\rContent-Length: 0\r\r')
  assertError(milo, parser)
})

// Obsolete folded headers are rejected.
it('compliance_obs_fold_rejected', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nHeader: value\r\n folded\r\nContent-Length: 0\r\n\r\n')
  assertError(milo, parser)
})

// Chunked transfer coding must be final.
it('compliance_chunked_must_be_final', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked, gzip\r\n\r\n')
  assertError(milo, parser)
})

// Content-Length cannot be combined with Transfer-Encoding.
it('compliance_content_length_transfer_encoding_conflict', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nContent-Length: 1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n')
  assertError(milo, parser)
})

// Connection close still finishes cleanly when no later data is received.
it('compliance_connection_close_finishes', t => {
  const { milo, parser, parse } = responseParser(t)
  parse('HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 0\r\n\r\n')
  assert.equal(milo.getState(parser), milo.STATE_FINISH)
})
