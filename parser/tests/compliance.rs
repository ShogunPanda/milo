mod helpers;

use milo_parser::{
  ERROR_NONE, ERROR_UNEXPECTED_TRAILERS, EVENT_ACTIVE_ON_TRAILER_NAME, EVENT_ACTIVE_ON_TRAILER_VALUE, EVENT_END,
  METHOD_OTHER, Parser, STATE_ERROR, STATE_FINISH, STATE_START, STATE_TUNNEL,
};

use crate::helpers::{create_parser, parse};

fn wire(input: &str) -> String { input.to_string() }

fn request_parser() -> Parser {
  let mut parser = create_parser();
  parser.autodetect = false;
  parser.is_request = true;
  parser
}

fn response_parser() -> Parser {
  let mut parser = create_parser();
  parser.autodetect = false;
  parser.is_request = false;
  parser
}

fn assert_ok(parser: &Parser) {
  assert_ne!(parser.state, STATE_ERROR, "{}", parser.error_description_str());
  assert_eq!(parser.error_code, ERROR_NONE);
}

fn assert_error(parser: &Parser) {
  assert_eq!(parser.state, STATE_ERROR);
}

#[test]
fn compliance_require_host_is_opt_in() {
  for request in ["GET / HTTP/1.1\r\n\r\n", "GET / HTTP/1.1\r\nHost: \t\r\n\r\n"] {
    let mut parser = request_parser();
    parse(&mut parser, request);
    assert_ok(&parser);
  }

  let mut parser = response_parser();
  parser.require_host = true;
  parse(&mut parser, "HTTP/1.1 204 No Content\r\n\r\n");
  assert_ok(&parser);

  let mut parser = request_parser();
  parser.require_host = true;
  parse(&mut parser, "PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n");
  assert_eq!(parser.state, STATE_TUNNEL);
  assert_ok(&parser);
}

#[test]
fn compliance_require_host_checks_initial_fields() {
  for (fields, error) in [
    ("", "Expected Host header"),
    ("Host:\r\n", "Expected Host header value"),
    ("Host: \t\r\n", "Expected Host header value"),
    ("Host: example.test\r\nhOsT: example.test\r\n", "Duplicate Host header"),
  ] {
    let mut parser = request_parser();
    parser.require_host = true;
    parse(&mut parser, &format!("GET / HTTP/1.1\r\n{fields}\r\n"));
    assert_error(&parser);
    assert_eq!(parser.error_description_str(), error);
  }

  let mut parser = request_parser();
  parser.require_host = true;
  let prefix = "GET http://example.test/ HTTP/1.1\r\nhOsT: not an authority\r\n";
  assert_eq!(parse(&mut parser, prefix), prefix.len());
  assert_eq!(parse(&mut parser, "\r\n"), 2);
  assert_ok(&parser);
  parse(&mut parser, "GET / HTTP/1.1\r\n\r\n");
  assert_error(&parser);
  assert_eq!(parser.error_description_str(), "Expected Host header");
}

// RFC token syntax allows `|` in header names.
#[test]
fn compliance_header_name_allows_pipe() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nX|Y: z\r\nContent-Length: 0\r\n\r\n");

  parse(&mut parser, &message);

  assert_ok(&parser);
}

// RFC token syntax rejects `,` in header names.
#[test]
fn compliance_header_name_rejects_comma() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nX,Y: z\r\nContent-Length: 0\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// RFC token syntax allows `|` in trailer names when chunked framing is valid.
#[test]
fn compliance_trailer_name_allows_pipe() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX|Y: z\r\n\r\n");

  parse(&mut parser, &message);

  assert_ok(&parser);
}

// RFC token syntax rejects `,` in trailer names.
#[test]
fn compliance_trailer_name_rejects_comma() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX,Y: z\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// RFC 9110 section 6.5.1 forbids trailer fields that affect framing,
// routing, authentication, connection handling, or payload processing.
#[test]
fn compliance_forbidden_trailer_fields_rejected() {
  for name in [
    "Accept-Ranges",
    "Age",
    "Allow",
    "Authorization",
    "Cache-Control",
    "Connection",
    "Content-Disposition",
    "Content-Encoding",
    "Content-Length",
    "Content-Range",
    "Content-Type",
    "Cookie",
    "Date",
    "ETag",
    "Expect",
    "Expires",
    "Forwarded",
    "Host",
    "If-Match",
    "If-Modified-Since",
    "If-None-Match",
    "If-Range",
    "If-Unmodified-Since",
    "Keep-Alive",
    "Last-Modified",
    "Location",
    "Max-Forwards",
    "Pragma",
    "Proxy-Authenticate",
    "Proxy-Authorization",
    "Proxy-Connection",
    "Range",
    "Referer",
    "Retry-After",
    "Server",
    "Set-Cookie",
    "TE",
    "Trailer",
    "Transfer-Encoding",
    "Upgrade",
    "Vary",
    "Via",
    "Warning",
    "WWW-Authenticate",
    "X-Forwarded-For",
    "X-Forwarded-Host",
    "X-Forwarded-Proto",
  ] {
    let mut parser = response_parser();
    let message = wire(&format!(
      "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: {name}\r\n\r\n0\r\n{name}: value\r\n\r\n"
    ));

    parse(&mut parser, &message);

    assert_error(&parser);
    assert_eq!(parser.error_code, ERROR_UNEXPECTED_TRAILERS);
  }
}

#[test]
fn compliance_forbidden_trailer_is_case_insensitive_and_not_delivered() {
  let mut parser = request_parser();
  parser.active_callbacks = 0;
  parser.active_events = EVENT_ACTIVE_ON_TRAILER_NAME | EVENT_ACTIVE_ON_TRAILER_VALUE;
  let message = wire(
    "POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\nTrailer: cOnTeNt-LeNgTh\r\n\r\n0\r\ncOnTeNt-LeNgTh: 1\r\n\r\n",
  );

  parse(&mut parser, &message);

  assert_error(&parser);
  assert_eq!(parser.error_code, ERROR_UNEXPECTED_TRAILERS);
  assert_eq!(parser.events()[0], EVENT_END);
}

#[test]
fn compliance_safe_trailer_field_accepted() {
  let mut parser = response_parser();
  let message =
    wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: Digest\r\n\r\n0\r\nDigest: sha-256=:abc:\r\n\r\n");

  parse(&mut parser, &message);

  assert_ok(&parser);
}

// Unknown valid method tokens are accepted as extension methods.
#[test]
fn compliance_unknown_method_token_is_accepted() {
  let mut parser = request_parser();
  let message = wire("FOO|BAR / HTTP/1.1\r\n\r\n");

  parse(&mut parser, &message);

  assert_ok(&parser);
  assert_eq!(parser.method, METHOD_OTHER);
}

// Invalid unknown method tokens are rejected.
#[test]
fn compliance_unknown_method_token_rejects_comma() {
  let mut parser = request_parser();
  let message = wire("BAD,METHOD / HTTP/1.1\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// PRI is only accepted with HTTP/2.0 for switch-over tunneling.
#[test]
fn compliance_pri_requires_http2() {
  let mut parser = request_parser();
  let message = wire("PRI * HTTP/1.1\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// PRI with HTTP/2.0 enters tunnel mode instead of parsing HTTP/1.1 headers.
#[test]
fn compliance_pri_http2_enters_tunnel() {
  let mut parser = request_parser();
  let message = wire("PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n");

  parse(&mut parser, &message);

  assert_eq!(parser.state, STATE_TUNNEL);
}

// HTTP/2.0 is rejected for normal requests.
#[test]
fn compliance_http2_request_rejected() {
  let mut parser = request_parser();
  let message = wire("GET / HTTP/2.0\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// HTTP/2.0 is rejected for responses.
#[test]
fn compliance_http2_response_rejected() {
  let mut parser = response_parser();
  let message = wire("HTTP/2.0 200 OK\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// PRI with HTTP/2.0 must be followed by the exact HTTP/2 connection preface
// suffix.
#[test]
fn compliance_pri_http2_invalid_preface_rejected() {
  let mut parser = request_parser();
  let message = wire("PRI * HTTP/2.0\r\ngarbage\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// RTSP is not detected or accepted as an HTTP response protocol.
#[test]
fn compliance_rtsp_response_rejected() {
  let mut parser = response_parser();
  let message = wire("RTSP/1.0 200 OK\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Request targets cannot contain fragments.
#[test]
fn compliance_request_target_rejects_fragment() {
  let mut parser = request_parser();
  let message = wire("GET /path#fragment HTTP/1.1\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Milo intentionally rejects bodies on GET requests.
#[test]
fn compliance_get_body_rejected() {
  let mut parser = request_parser();
  let message = wire("GET / HTTP/1.1\r\nContent-Length: 1\r\n\r\nx");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Milo intentionally rejects bodies on HEAD requests.
#[test]
fn compliance_head_body_rejected() {
  let mut parser = request_parser();
  let message = wire("HEAD / HTTP/1.1\r\nContent-Length: 1\r\n\r\nx");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Milo rejects chunked framing on GET and HEAD even when the encoded body is empty.
#[test]
fn compliance_get_head_chunked_body_rejected() {
  for method in ["GET", "HEAD"] {
    for body in ["0\r\n\r\n", "1\r\nx\r\n0\r\n\r\n"] {
      let mut parser = request_parser();
      let message = wire(&format!(
        "{method} / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n{body}"
      ));

      parse(&mut parser, &message);

      assert_error(&parser);
    }
  }
}

// CONNECT switches immediately to tunnel mode, so any request-body framing
// would make intermediaries disagree about where the tunnel data begins.
#[test]
fn compliance_connect_content_length_rejected() {
  let mut parser = request_parser();
  let message = wire("CONNECT example.com:443 HTTP/1.1\r\nContent-Length: 0\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

#[test]
fn compliance_connect_transfer_encoding_rejected() {
  let mut parser = request_parser();
  let message = wire("CONNECT example.com:443 HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Methods other than GET, HEAD, and CONNECT can carry valid body framing.
#[test]
fn compliance_post_body_accepted() {
  let mut parser = request_parser();
  let message = wire("POST / HTTP/1.1\r\nContent-Length: 1\r\n\r\nx");

  parse(&mut parser, &message);

  assert_ok(&parser);
}

// 205 responses complete after headers like other no-body statuses.
#[test]
fn compliance_205_without_body_completes() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 205 Reset Content\r\n\r\n");

  parse(&mut parser, &message);

  assert_eq!(parser.state, STATE_START);
}

// 205 responses reject Content-Length as body framing.
#[test]
fn compliance_205_content_length_rejected() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 205 Reset Content\r\nContent-Length: 0\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// 205 responses reject Transfer-Encoding as body framing.
#[test]
fn compliance_205_transfer_encoding_rejected() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 205 Reset Content\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

#[test]
fn compliance_1xx_content_length_rejected() {
  let mut parser = response_parser();

  parse(&mut parser, "HTTP/1.1 100 Continue\r\nContent-Length: 0\r\n\r\n");

  assert_error(&parser);
}

// 304 allows Content-Length as metadata but still has no body.
#[test]
fn compliance_304_content_length_accepted_without_body() {
  let mut parser = response_parser();
  let message =
    wire("HTTP/1.1 304 Not Modified\r\nContent-Length: 10\r\n\r\nHTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");

  parse(&mut parser, &message);

  assert_ok(&parser);
  assert_eq!(parser.state, STATE_START);
}

#[test]
fn compliance_1xx_transfer_encoding_rejected() {
  let mut parser = response_parser();
  let message = "HTTP/1.1 100 Continue\r\nTransfer-Encoding: chunked\r\n\r\n";

  parse(&mut parser, message);

  assert_error(&parser);
}

// Trailer is invalid without chunked transfer coding.
#[test]
fn compliance_trailer_without_chunked_rejected() {
  let mut parser = request_parser();
  let message = wire("POST / HTTP/1.1\r\nTrailer: X\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Upgrade does not bypass Trailer validation.
#[test]
fn compliance_upgrade_trailer_without_chunked_rejected() {
  let mut parser = request_parser();
  let message = wire("POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: websocket\r\nTrailer: X\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Request upgrade may parse a chunked body and trailers before tunneling.
#[test]
fn compliance_request_upgrade_chunked_trailers_before_tunnel() {
  let mut parser = request_parser();
  let message = wire(
    "POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: websocket\r\nTransfer-Encoding: chunked\r\nTrailer: \
     X\r\n\r\n0\r\nX: y\r\n\r\n",
  );

  parse(&mut parser, &message);

  assert_eq!(parser.state, STATE_TUNNEL);
}

// Response upgrade cannot use Trailer without valid chunked framing.
#[test]
fn compliance_response_upgrade_trailer_rejected() {
  let mut parser = response_parser();
  let message =
    wire("HTTP/1.1 101 Switching Protocols\r\nConnection: upgrade\r\nUpgrade: websocket\r\nTrailer: X\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Valid unknown Connection options are accepted and ignored.
#[test]
fn compliance_connection_unknown_token_accepted() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nConnection: foo\r\nContent-Length: 0\r\n\r\n");

  parse(&mut parser, &message);

  assert_ok(&parser);
}

// Invalid unknown Connection options are rejected.
#[test]
fn compliance_connection_unknown_token_rejected() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nConnection: foo@bar\r\nContent-Length: 0\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Empty Connection list items are rejected.
#[test]
fn compliance_connection_empty_item_rejected() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nConnection: close,,upgrade\r\nContent-Length: 0\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Connection close finishes the parser and rejects subsequent data.
#[test]
fn compliance_connection_close_rejects_later_data() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 0\r\n\r\nx");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Upgrade values are comma-separated protocol tokens without special known
// values.
#[test]
fn compliance_upgrade_tokens_accepted() {
  let mut parser = request_parser();
  let message = wire("POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: foo, HTTP/2.0\r\n\r\n");

  parse(&mut parser, &message);

  assert_eq!(parser.state, STATE_TUNNEL);
}

// Upgrade protocol values reject empty protocol names.
#[test]
fn compliance_upgrade_empty_protocol_name_rejected() {
  let mut parser = request_parser();
  let message = wire("POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: /2.0\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Upgrade protocol values reject empty protocol versions.
#[test]
fn compliance_upgrade_empty_protocol_version_rejected() {
  let mut parser = request_parser();
  let message = wire("POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: HTTP/\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Upgrade protocol values reject more than one protocol version separator.
#[test]
fn compliance_upgrade_extra_protocol_separator_rejected() {
  let mut parser = request_parser();
  let message = wire("POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: HTTP/2/extra\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Empty Upgrade values are rejected.
#[test]
fn compliance_upgrade_empty_rejected() {
  let mut parser = request_parser();
  let message = wire("POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: \r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Invalid Upgrade tokens are rejected.
#[test]
fn compliance_upgrade_invalid_token_rejected() {
  let mut parser = request_parser();
  let message = wire("POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: foo@bar\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Empty Upgrade list items are rejected.
#[test]
fn compliance_upgrade_empty_item_rejected() {
  let mut parser = request_parser();
  let message = wire("POST / HTTP/1.1\r\nConnection: upgrade\r\nUpgrade: foo,,bar\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Unquoted chunk extension values must be RFC tokens.
#[test]
fn compliance_chunk_extension_unquoted_token_value() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo=bar|baz\r\nx\r\n0\r\n\r\n");

  parse(&mut parser, &message);

  assert_ok(&parser);
}

// Unquoted chunk extension values reject spaces.
#[test]
fn compliance_chunk_extension_unquoted_space_rejected() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo=bar baz\r\nx\r\n0\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Unquoted chunk extension values reject non-token characters.
#[test]
fn compliance_chunk_extension_unquoted_at_rejected() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo=bar@baz\r\nx\r\n0\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Quoted chunk extension values may contain spaces.
#[test]
fn compliance_chunk_extension_quoted_space_accepted() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo=\"bar baz\"\r\nx\r\n0\r\n\r\n");

  parse(&mut parser, &message);
  assert_ok(&parser);
}

// Quoted chunk extension values may contain quoted-pair escaped quotes.
#[test]
fn compliance_chunk_extension_quoted_escaped_quote_accepted() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo=\"bar\\\"baz\"\r\nx\r\n0\r\n\r\n");

  parse(&mut parser, &message);
  assert_ok(&parser);
}

// Quoted chunk extension values may contain quoted-pair escaped backslashes.
#[test]
fn compliance_chunk_extension_quoted_escaped_backslash_accepted() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo=\"bar\\\\baz\"\r\nx\r\n0\r\n\r\n");

  parse(&mut parser, &message);
  assert_ok(&parser);
}

// Quoted chunk extension values may contain horizontal tabs.
#[test]
fn compliance_chunk_extension_quoted_tab_accepted() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo=\"bar\tbaz\"\r\nx\r\n0\r\n\r\n");

  parse(&mut parser, &message);
  assert_ok(&parser);
}

// Quoted chunk extension values may contain obs-text.
#[test]
fn compliance_chunk_extension_quoted_obs_text_accepted() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo=\"bar\u{80}baz\"\r\nx\r\n0\r\n\r\n");

  parse(&mut parser, &message);
  assert_ok(&parser);
}

// Quoted chunk extension values reject bare control characters other than HTAB.
#[test]
fn compliance_chunk_extension_quoted_control_rejected() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo=\"bar\u{1}baz\"\r\nx\r\n0\r\n\r\n");

  parse(&mut parser, &message);
  assert_error(&parser);
}

// Quoted-pair in chunk extension values rejects escaped control characters.
#[test]
fn compliance_chunk_extension_quoted_pair_control_rejected() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;foo=\"bar\\\u{1}baz\"\r\nx\r\n0\r\n\r\n");

  parse(&mut parser, &message);
  assert_error(&parser);
}

// Bare CR is rejected in HTTP framing.
#[test]
fn compliance_bare_cr_rejected() {
  let mut parser = response_parser();
  let message = "HTTP/1.1 200 OK\rContent-Length: 0\r\r";

  parse(&mut parser, message);

  assert_error(&parser);
}

// Obsolete folded headers are rejected.
#[test]
fn compliance_obs_fold_rejected() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nHeader: value\r\n folded\r\nContent-Length: 0\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Chunked transfer coding must be final.
#[test]
fn compliance_chunked_must_be_final() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked, gzip\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Transfer-coding names use HTTP token syntax.
#[test]
fn compliance_transfer_encoding_rejects_invalid_coding_name() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: bad@coding, chunked\r\n\r\n0\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
  assert!(!parser.has_transfer_encoding);
  assert!(!parser.has_chunked_transfer_encoding);
}

// Transfer-Encoding list members cannot be empty.
#[test]
fn compliance_transfer_encoding_rejects_empty_list_members() {
  for value in [", chunked", "gzip,, chunked", "gzip, chunked,"] {
    let mut parser = response_parser();
    let message = wire(&format!(
      "HTTP/1.1 200 OK\r\nTransfer-Encoding: {value}\r\n\r\n0\r\n\r\n"
    ));

    parse(&mut parser, &message);

    assert_error(&parser);
  }
}

// Transfer parameters require token names and token or quoted-string values.
#[test]
fn compliance_transfer_encoding_rejects_invalid_parameters() {
  for value in [
    "gzip; level, chunked",
    "gzip; =9, chunked",
    "gzip; level=, chunked",
    "gzip; level=bad@value, chunked",
    "gzip; level=\"unterminated, chunked",
    "gzip; level=\"bad\u{1}\", chunked",
    "chunked; level=9",
  ] {
    let mut parser = response_parser();
    let message = wire(&format!(
      "HTTP/1.1 200 OK\r\nTransfer-Encoding: {value}\r\n\r\n0\r\n\r\n"
    ));

    parse(&mut parser, &message);

    assert_error(&parser);
  }
}

// Valid non-chunked parameters are preserved, including quoted commas, before
// final chunked.
#[test]
fn compliance_transfer_encoding_accepts_parameters_before_chunked() {
  let mut parser = response_parser();
  let message =
    wire("HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip; level = 9; note=\"a,b\\\"c\\\\d\", ChUnKeD\r\n\r\n0\r\n\r\n");

  parse(&mut parser, &message);

  assert_ok(&parser);
  assert!(parser.has_transfer_encoding);
  assert!(parser.has_chunked_transfer_encoding);
}

// Chunked can occur only once, even across multiple field lines.
#[test]
fn compliance_transfer_encoding_rejects_duplicate_chunked() {
  for headers in [
    "Transfer-Encoding: gzip, chunked, chunked\r\n",
    "Transfer-Encoding: gzip, chunked\r\nTransfer-Encoding: chunked\r\n",
  ] {
    let mut parser = response_parser();
    let message = wire(&format!("HTTP/1.1 200 OK\r\n{headers}\r\n0\r\n\r\n"));

    parse(&mut parser, &message);

    assert_error(&parser);
  }
}

// Content-Length cannot be combined with Transfer-Encoding.
#[test]
fn compliance_content_length_transfer_encoding_conflict() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nContent-Length: 1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n");

  parse(&mut parser, &message);

  assert_error(&parser);
}

// Connection close still finishes cleanly when no later data is received.
#[test]
fn compliance_connection_close_finishes() {
  let mut parser = response_parser();
  let message = wire("HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 0\r\n\r\n");

  parse(&mut parser, &message);

  assert_eq!(parser.state, STATE_FINISH);
}
