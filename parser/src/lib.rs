#[macro_use]
extern crate lazy_static;

use std::ffi::CString;
use std::fmt::{Debug, Display, Formatter, Result};
use std::os::raw::{c_char, c_uchar, c_void};
use std::ptr;
use std::slice;
use std::str;
#[cfg(all(debug_assertions, feature = "milo_debug_loop"))]
use std::time::Instant;

use milo_parser_macros::{
  apply_state, c_callbacks_setters, c_match_error_code_string, c_match_state_string, c_spans_getters, c_values_getters,
  c_values_setters, callback, callbacks, case_insensitive_string, char, consume, crlf, digit, double_crlf, errors,
  fail, find_method, generate_parser, hex_digit, initial_state, method, move_to, otherwise, persistent_values, spans,
  state, string, string_length, suspend, token, token_value, url, user_writable_values, values,
};

pub mod test_utils;

pub const AUTODETECT: isize = 0;
pub const REQUEST: isize = 1;
pub const RESPONSE: isize = 2;

pub const CONNECTION_KEEPALIVE: isize = 0;
pub const CONNECTION_CLOSE: isize = 1;
pub const CONNECTION_UPGRADE: isize = 2;

values!(
  id,
  mode,
  continue_without_data,
  message_type,
  is_connect_request,
  method,
  status,
  version_major,
  version_minor,
  connection,
  content_length,
  chunk_size,
  has_content_length,
  has_chunked_transfer_encoding,
  has_upgrade,
  has_trailers,
  skip_body
);

user_writable_values!(id, mode, is_connect_request, skip_body);

persistent_values!(id, mode, continue_without_data);

spans!(unconsumed, body);

errors!(
  NONE,
  UNEXPECTED_DATA,
  UNEXPECTED_EOF,
  CALLBACK_ERROR,
  UNEXPECTED_CHARACTER,
  UNEXPECTED_CONTENT_LENGTH,
  UNEXPECTED_TRANSFER_ENCODING,
  UNEXPECTED_CONTENT,
  UNTRAILERS,
  INVALID_VERSION,
  INVALID_STATUS,
  INVALID_CONTENT_LENGTH,
  INVALID_TRANSFER_ENCODING,
  INVALID_CHUNK_SIZE,
  MISSING_CONNECTION_UPGRADE,
  UNSUPPORTED_HTTP_VERSION
);

callbacks!(
  after_state_change,
  before_state_change,
  on_error,
  on_finish,
  on_message_start,
  on_message_complete,
  on_request,
  on_response,
  on_reset,
  on_method,
  on_url,
  on_protocol,
  on_version,
  on_status,
  on_reason,
  on_header_name,
  on_header_value,
  on_headers,
  on_connect,
  on_upgrade,
  on_chunk_length,
  on_chunk_extension_name,
  on_chunk_extension_value,
  on_chunk_data,
  on_body,
  on_data,
  on_trailer_name,
  on_trailer_value,
  on_trailers
);

#[inline(always)]
fn store_parsed_http_version(parser: &mut Parser, major: c_uchar) {
  if major == char!('1') {
    parser.values.version_major = 1;
    parser.values.version_minor = 1;
  } else {
    parser.values.version_major = 2;
    parser.values.version_minor = 0;
  }
}

// #region general
// Depending on the mode flag, choose the initial state
state!(start, {
  match parser.values.mode {
    AUTODETECT => move_to!(message, 0),
    REQUEST => {
      parser.values.message_type = REQUEST;
      callback!(on_message_start);
      callback!(on_request);
      move_to!(request, 0)
    }
    RESPONSE => {
      parser.values.message_type = RESPONSE;
      callback!(on_message_start);
      callback!(on_response);
      move_to!(response, 0)
    }
    _ => fail!(UNEXPECTED_CHARACTER, "Invalid mode"),
  }
});

state!(finish, { 0 });

state!(error, { 0 });

// Autodetect if there is a HTTP/RTSP method or a response
state!(message, {
  match data {
    crlf!() => 2, // RFC 9112 section 2.2,
    string!("HTTP/") | string!("RTSP/") => {
      parser.values.message_type = RESPONSE;
      callback!(on_message_start);
      callback!(on_response);
      move_to!(response, 0)
    }
    method!() => {
      parser.values.message_type = REQUEST;
      callback!(on_message_start);
      callback!(on_request);
      move_to!(request, 0)
    }
    otherwise!(5) => fail!(UNEXPECTED_CHARACTER, "Unexpected data"),
    _ => suspend!(),
  }
});

state!(end, {
  let must_close = parser.values.connection == CONNECTION_CLOSE;
  parser.values.connection = 0;

  if must_close {
    move_to!(finish, 0)
  } else {
    move_to!(start, 0)
  }
});
// #general

// #region request
// RFC 9112 section 3
state!(request, {
  match data {
    crlf!() => 2, // RFC 9112 section 2.2 - Repeated
    [token!(), ..] => move_to!(request_method, 0),
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Expected method"),
    _ => suspend!(),
  }
});

// RFC 9112 section 3.1
state!(request_method, {
  consume!(token!());

  match data[consumed] {
    char!(' ') if consumed > 0 => {
      find_method!(&data[..consumed]);
      parser.values.method = method;

      callback!(on_method, consumed);
      move_to!(request_url, consumed + 1)
    }
    _ => fail!(UNEXPECTED_CHARACTER, "Expected token character"),
  }
});

// RFC 9112 section 3.2
state!(request_url, {
  consume!(url!());

  match data[consumed] {
    char!(' ') if consumed > 0 => {
      callback!(on_url, consumed);
      move_to!(request_protocol, consumed + 1)
    }
    _ => fail!(UNEXPECTED_CHARACTER, "Expected URL character"),
  }
});

// RFC 9112 section 2.3
state!(request_protocol, {
  match data {
    string!("HTTP/") | string!("RTSP/") => {
      callback!(on_protocol, 4);
      parser.position += 4;

      move_to!(request_version, 1)
    }
    otherwise!(5) => fail!(UNEXPECTED_CHARACTER, "Expected protocol"),
    _ => suspend!(),
  }
});

state!(request_version, {
  match data {
    [digit!(), char!('.'), digit!(), char!('\r'), char!('\n'), ..] => {
      // Validate the version
      let version = &data[0..3];

      match version {
        string!("1.1") | string!("2.0") => {
          store_parsed_http_version(parser, data[0]);

          if parser.values.method == METHOD_PRI {
            return fail!(UNSUPPORTED_HTTP_VERSION, "HTTP/2.0 is not supported");
          }

          callback!(on_version, 3);
          move_to!(header_name, 5)
        }
        _ => fail!(INVALID_VERSION, "Invalid HTTP version"),
      }
    }
    otherwise!(5) => fail!(UNEXPECTED_CHARACTER, "Expected HTTP version"),
    _ => suspend!(),
  }
});
// #endregion request

// #region response
// RFC 9112 section 4
state!(response, {
  match data {
    crlf!() => 2, // RFC 9112 section 2.2 - Repeated
    string!("HTTP/") | string!("RTSP/") => {
      callback!(on_protocol, 4);
      move_to!(response_version, 5)
    }
    otherwise!(5) => {
      fail!(UNEXPECTED_CHARACTER, "Expected protocol")
    }
    _ => suspend!(),
  }
});

state!(response_version, {
  match data {
    [digit!(), char!('.'), digit!(), char!(' '), ..] => {
      // Validate the version
      let version = &data[0..3];

      match version {
        string!("1.1") | string!("2.0") => {
          store_parsed_http_version(parser, data[0]);
          callback!(on_version, 3);
          move_to!(response_status, 4)
        }
        _ => fail!(INVALID_VERSION, "Invalid HTTP version"),
      }
    }
    otherwise!(4) => fail!(UNEXPECTED_CHARACTER, "Expected HTTP version"),
    _ => suspend!(),
  }
});

state!(response_status, {
  // Collect the three digits
  match data {
    [digit!(), digit!(), digit!(), char!(' '), ..] => {
      parser.values.status = isize::from_str_radix(unsafe { str::from_utf8_unchecked(&data[0..3]) }, 10).unwrap();
      callback!(on_status, 3);
      move_to!(response_reason, 4)
    }
    otherwise!(4) => fail!(INVALID_STATUS, "Expected HTTP response status"),
    _ => suspend!(),
  }
});

state!(response_reason, {
  consume!(token_value!());

  match data[consumed..] {
    crlf!() => {
      if consumed > 0 {
        callback!(on_reason, consumed);
        parser.position += consumed;
      }

      move_to!(header_name, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Expected status reason"),
    _ => suspend!(),
  }
});
// #endregion response

// #region headers
// RFC 9112 section 4
state!(header_name, {
  // Special headers treating
  match data {
    case_insensitive_string!("content-length:") => {
      let status = parser.values.status;

      if parser.values.has_chunked_transfer_encoding == 1 {
        return fail!(
          UNEXPECTED_CONTENT_LENGTH,
          "Unexpected Content-Length header when Transfer-Encoding header is present"
        );
      } else if parser.values.status == 204 || parser.values.status / 100 == 1 {
        return fail!(
          UNEXPECTED_CONTENT_LENGTH,
          format!("Unexpected Content-Length header for a response with status {}", status)
        );
      } else if parser.values.content_length != 0 {
        return fail!(INVALID_CONTENT_LENGTH, "Invalid duplicate Content-Length header");
      }

      parser.values.has_content_length = 1;
      callback!(on_header_name, string_length!("content-length"));
      return move_to!(header_content_length, string_length!("content-length", 1));
    }
    case_insensitive_string!("transfer-encoding:") => {
      let status = parser.values.status;

      if parser.values.content_length > 0 {
        return fail!(
          UNEXPECTED_TRANSFER_ENCODING,
          "Unexpected Transfer-Encoding header when Content-Length header is present"
        );
      } else if parser.values.status == 304 {
        // Transfer-Encoding is NOT allowed in 304
        return fail!(
          UNEXPECTED_TRANSFER_ENCODING,
          format!(
            "Unexpected Transfer-Encoding header for a response with status {}",
            status
          )
        );
      }

      callback!(on_header_name, string_length!("transfer-encoding"));
      return move_to!(header_transfer_encoding, string_length!("transfer-encoding", 1));
    }
    case_insensitive_string!("connection:") => {
      callback!(on_header_name, string_length!("connection"));
      return move_to!(header_connection, string_length!("connection", 1));
    }
    // RFC 9110 section 9.5
    case_insensitive_string!("trailer:") => {
      parser.values.has_trailers = 1;
      callback!(on_header_name, string_length!("trailer"));
      return move_to!(header_value, string_length!("trailer", 1));
    }
    // RFC 9110 section 7.8
    case_insensitive_string!("upgrade:") => {
      parser.values.has_upgrade = 1;
      callback!(on_header_name, string_length!("upgrade"));
      return move_to!(header_value, string_length!("upgrade", 1));
    }
    _ => {}
  }

  consume!(token!());

  match data[consumed..] {
    [char!(':'), ..] if consumed > 0 => {
      callback!(on_header_name, consumed);
      move_to!(header_value, consumed + 1)
    }
    crlf!() => {
      parser.values.continue_without_data = 1;
      move_to!(headers, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid header field name character"),
    _ => suspend!(),
  }
});

// RFC 9112 section 6.1
state!(header_transfer_encoding, {
  // Ignore trailing OWS
  consume!(char!('\t') | char!(' '));
  parser.position += consumed;
  data = &data[consumed..];

  if let case_insensitive_string!("chunked\r\n")
  | case_insensitive_string!(",chunked\r\n")
  | case_insensitive_string!(", chunked\r\n") = data
  {
    // If this is 1, it means the Transfer-Encoding header was specified more than
    // once. This is the second repetition and therefore, the previous one is no
    // longer the last one, making it invalid.
    if parser.values.has_chunked_transfer_encoding == 1 {
      return fail!(
        INVALID_TRANSFER_ENCODING,
        "The value \"chunked\" in the Transfer-Encoding header must be the last provided and can be provided only once"
      );
    }

    parser.values.has_chunked_transfer_encoding = 1;
  } else if parser.values.has_chunked_transfer_encoding == 1 {
    // Any other value when chunked was already specified is invalid as the previous
    // chunked would not be the last one anymore
    return fail!(
      INVALID_TRANSFER_ENCODING,
      "The value \"chunked\" in the Transfer-Encoding header must be the last provided"
    );
  }

  consume!(token_value!());

  if consumed == 0 {
    return fail!(UNEXPECTED_CHARACTER, "Invalid header field value character");
  }

  match data[consumed..] {
    double_crlf!() => {
      callback!(on_header_value, consumed);
      parser.position += consumed;
      parser.values.continue_without_data = 1;
      move_to!(headers, 4)
    }
    crlf!() => {
      callback!(on_header_value, consumed);
      move_to!(header_name, consumed + 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid header field value character"),
    _ => suspend!(),
  }
});

// RFC 9112 section 6.2
state!(header_content_length, {
  // Ignore trailing OWS
  consume!(char!('\t') | char!(' '));
  parser.position += consumed;
  data = &data[consumed..];

  consume!(digit!());

  if consumed == 0 {
    return fail!(UNEXPECTED_CHARACTER, "Invalid header field value character");
  }

  match data[consumed..] {
    crlf!() => {
      if let Ok(length) = unsafe { str::from_utf8_unchecked(&data[0..consumed]) }.parse::<usize>() {
        parser.values.content_length = length as isize;

        callback!(on_header_value, consumed);
        return move_to!(header_name, consumed + 2);
      } else {
        fail!(INVALID_CONTENT_LENGTH, "Invalid Content-Length header")
      }
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid header field value character"),
    _ => suspend!(),
  }
});

// RFC 9112 section 9.6
state!(header_connection, {
  // Ignore trailing OWS
  consume!(char!('\t') | char!(' '));
  parser.position += consumed;
  data = &data[consumed..];

  match data {
    case_insensitive_string!("close\r\n") => {
      parser.values.connection = CONNECTION_CLOSE;
      callback!(on_header_value, string_length!("close"));
      return move_to!(header_name, string_length!("close", 2));
    }
    case_insensitive_string!("keep-alive\r\n") => {
      parser.values.connection = CONNECTION_KEEPALIVE;
      callback!(on_header_value, string_length!("keep-alive"));
      return move_to!(header_name, string_length!("keep-alive", 2));
    }
    case_insensitive_string!("upgrade\r\n") => {
      parser.values.connection = CONNECTION_UPGRADE;
      callback!(on_header_value, string_length!("upgrade"));
      return move_to!(header_name, string_length!("upgrade", 2));
    }
    _ => {}
  }

  consume!(token_value!());

  if consumed == 0 {
    return fail!(UNEXPECTED_CHARACTER, "Invalid header field value character");
  }

  match data[consumed..] {
    double_crlf!() => {
      callback!(on_header_value, consumed);
      parser.position += consumed;
      parser.values.continue_without_data = 1;
      move_to!(headers, 4)
    }
    crlf!() => {
      callback!(on_header_value, consumed);
      move_to!(header_name, consumed + 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid header field value character"),
    _ => suspend!(),
  }
});

// RFC 9110 section 5.5 and 5.6
state!(header_value, {
  // Ignore trailing OWS
  consume!(char!('\t') | char!(' '));
  parser.position += consumed;
  data = &data[consumed..];

  consume!(token_value!());

  if consumed == 0 {
    return fail!(UNEXPECTED_CHARACTER, "Invalid header field value character");
  }

  match data[consumed..] {
    double_crlf!() => {
      callback!(on_header_value, consumed);
      parser.position += consumed;
      parser.values.continue_without_data = 1;
      move_to!(headers, 4)
    }
    crlf!() => {
      callback!(on_header_value, consumed);
      move_to!(header_name, consumed + 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid header field value character"),
    _ => suspend!(),
  }
});

// RFC 9110 section 9.3.6 and 7.8
state!(headers, {
  parser.values.continue_without_data = 1;

  if parser.values.has_upgrade == 1 && parser.values.connection != CONNECTION_UPGRADE {
    parser.values.continue_without_data = 0;

    return fail!(
      MISSING_CONNECTION_UPGRADE,
      format!("Missing Connection header set to \"upgrade\" when using the Upgrade header")
    );
  }

  callback!(on_headers);

  let method = parser.values.method;
  let status = parser.values.status;

  // In case of Connection: Upgrade
  if parser.values.has_upgrade == 1 {
    if parser.values.connection != CONNECTION_UPGRADE {
      parser.values.continue_without_data = 0;

      return fail!(
        MISSING_CONNECTION_UPGRADE,
        format!("Missing Connection header set to \"upgrade\" when using the Upgrade header")
      );
    }

    callback!(on_upgrade);
    return move_to!(tunnel, 0);
  }

  // In case of CONNECT method
  if parser.values.is_connect_request == 1 {
    callback!(on_connect);
    return move_to!(tunnel, 0);
  }

  if method == METHOD_GET || method == METHOD_HEAD {
    if parser.values.content_length > 0 {
      parser.values.continue_without_data = 0;

      return fail!(UNEXPECTED_CONTENT, format!("Unexpected content for {} request", method));
    }
  }

  // RFC 9110 section 6.3
  if parser.values.message_type == REQUEST {
    if parser.values.has_content_length == 1 {
      if parser.values.content_length == 0 {
        return complete_message(parser, 0);
      }
    } else if parser.values.has_chunked_transfer_encoding == 0 {
      return complete_message(parser, 0);
    }
  } else {
    if (status < 200 && status != 101) || method == METHOD_HEAD || parser.values.skip_body == 1 {
      return complete_message(parser, 0);
    }

    if parser.values.content_length == 0 {
      if parser.values.has_content_length == 1 {
        return complete_message(parser, 0);
      } else if parser.values.has_chunked_transfer_encoding == 0 {
        return move_to!(body_with_no_length, 0);
      }
    }
  }

  move_to!(body, 0)
});

// #endregion headers

// RFC 9110 section 6.4.1
#[inline(always)]
fn complete_message(parser: &mut Parser, advance: isize) -> isize {
  callback!(on_message_complete);

  let connection = parser.values.connection;

  parser.values.clear();
  parser.spans.body.clear();
  parser.values.continue_without_data = 1;
  parser.values.connection = connection;

  move_to!(end, advance)
}

// #region common_body
state!(body, {
  if parser.values.content_length > 0 {
    return move_to!(body_via_content_length, 0);
  }

  if parser.values.has_trailers == 1 && !parser.values.has_chunked_transfer_encoding == 0 {
    return fail!(
      UNTRAILERS,
      "Trailers are not allowed when not using chunked transfer encoding"
    );
  }

  move_to!(chunk_length, 0)
});

// Return PAUSE makes this method idempotent without failing - In this state all
// data is ignored since we're not in HTTP anymore
state!(tunnel, { suspend!() });

// #endregion common_body

// #region body via Content-Length
// RFC 9112 section 6.2
state!(body_via_content_length, {
  let expected = parser.values.content_length as usize;
  let available = data.len();

  // Less data than what we expect
  if available < expected {
    return suspend!();
  }

  callback!(on_data, expected);
  callback!(on_body, expected);
  complete_message(parser, expected as isize)
});
// #endregion body via Content-Length

// RFC 9110 section 6.3
// Note that on_body can't and will not be called here as there is no way to
// know when the response finishes
state!(body_with_no_length, {
  let len = data.len();
  callback!(on_data, len);
  0
});

// #region body via chunked Transfer-Encoding
// RFC 9112 section 7.1
state!(chunk_length, {
  consume!(hex_digit!());

  match data[consumed..] {
    [char!(';'), ..] if consumed > 0 => {
      if let Ok(length) = usize::from_str_radix(unsafe { str::from_utf8_unchecked(&data[..consumed]) }, 16) {
        callback!(on_chunk_length, consumed);
        parser.values.chunk_size = length as isize;
        move_to!(chunk_extension_name, consumed + 1)
      } else {
        fail!(INVALID_CHUNK_SIZE, "Invalid chunk length")
      }
    }
    crlf!() => {
      if let Ok(length) = usize::from_str_radix(unsafe { str::from_utf8_unchecked(&data[..consumed]) }, 16) {
        callback!(on_chunk_length, consumed);
        parser.values.chunk_size = length as isize;
        parser.values.continue_without_data = 1;
        move_to!(chunk_data, consumed + 2)
      } else {
        fail!(INVALID_CHUNK_SIZE, "Invalid chunk length")
      }
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid chunk length character"),
    _ => suspend!(),
  }
});

state!(chunk_extension_name, {
  consume!(token!());

  if consumed == 0 {
    return fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension name character");
  }

  match data[consumed..] {
    [char!('='), ..] => {
      callback!(on_chunk_extension_name, consumed);
      move_to!(chunk_extension_value, consumed + 1)
    }
    [char!(';'), ..] => {
      callback!(on_chunk_extension_name, consumed);
      move_to!(chunk_extension_name, consumed + 1)
    }
    crlf!() => {
      callback!(on_chunk_extension_name, consumed);

      parser.values.continue_without_data = 1;
      move_to!(chunk_data, consumed + 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension name character"),
    _ => suspend!(),
  }
});

state!(chunk_extension_value, {
  if data[0] == char!('"') {
    return move_to!(chunk_extension_quoted_value, 1);
  }

  consume!(token!());

  if consumed == 0 {
    return fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension value character");
  }

  match data[consumed..] {
    [char!(';'), ..] => {
      callback!(on_chunk_extension_value, consumed);
      move_to!(chunk_extension_name, consumed + 1)
    }
    crlf!() => {
      callback!(on_chunk_extension_value, consumed);
      parser.values.continue_without_data = 1;
      move_to!(chunk_data, consumed + 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension value character"),
    _ => suspend!(),
  }
});

// RFC 9110 section 5.6.4
state!(chunk_extension_quoted_value, {
  // Also consume 0x22 and 0x5c as the quoted-pair validation is performed after
  consume!(char!('\t') | char!(' ') | 0x21..=0x7e);

  if consumed == 0 || data[consumed - 1] != char!('"') {
    return fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension quoted value");
  }

  // Search if multiple parameters are specified on the same line. Stop on the
  // first non quoted "
  for i in 0..consumed - 2 {
    if data[i + 1] == char!('"') && data[i] != char!('\\') {
      consumed = i + 2;
      break;
    }
  }

  // If the last " is quoted, then fail
  if data[consumed - 2] == char!('\\') && data[consumed - 1] == char!('"') {
    return fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension quoted value");
  }

  match data[consumed..] {
    crlf!() => {
      parser.values.continue_without_data = 1;
      callback!(on_chunk_extension_value, consumed - 1);
      move_to!(chunk_data, consumed + 2)
    }
    [char!(';'), ..] => {
      parser.values.continue_without_data = 1;
      callback!(on_chunk_extension_value, consumed - 1);
      move_to!(chunk_extension_name, consumed + 2)
    }
    otherwise!(3) => {
      fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension quoted value character")
    }
    _ => suspend!(),
  }
});

state!(chunk_data, {
  if parser.values.chunk_size == 0 {
    let body = &parser.spans.body;

    if (parser.callbacks.on_body)(parser, body.as_ptr(), body.len()) < 0 {
      return fail!(CALLBACK_ERROR, "Callback returned an error.");
    }

    if parser.values.has_trailers == 1 {
      return move_to!(trailer_name, 0);
    } else {
      return move_to!(crlf_after_last_chunk, 0);
    }
  }

  let expected = parser.values.chunk_size as usize;
  let available = data.len();

  // Less data than what we expect for this chunk
  if available < expected {
    return suspend!();
  }

  let chunk_data = data.get(..expected).unwrap();

  parser.spans.body.extend_from_slice(chunk_data);
  callback!(on_data, expected);
  callback!(on_chunk_data, expected);
  move_to!(chunk_end, expected)
});

state!(chunk_end, {
  match data {
    crlf!() => {
      parser.values.chunk_size = 0;
      move_to!(chunk_length, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Unexpected character after chunk data"),
    _ => suspend!(),
  }
});

state!(crlf_after_last_chunk, {
  match data {
    crlf!() => {
      parser.values.continue_without_data = 1;
      complete_message(parser, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Expected CRLF after the last chunk"),
    _ => suspend!(),
  }
});

// #endregion body via chunked Transfer-Encoding

// #region trailers
// RFC 9112 section 7.1.2
state!(trailer_name, {
  consume!(token!());

  match data[consumed..] {
    [char!(':'), ..] if consumed > 0 => {
      callback!(on_trailer_name, consumed);
      move_to!(trailer_value, consumed + 1)
    }
    crlf!() => {
      callback!(on_trailers);
      complete_message(parser, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid trailer field name character"),
    _ => suspend!(),
  }
});

state!(trailer_value, {
  // Ignore trailing OWS
  consume!(char!('\t') | char!(' '));
  parser.position += consumed;
  data = &data[consumed..];

  consume!(token_value!());

  if consumed == 0 {
    return fail!(UNEXPECTED_CHARACTER, "Invalid trailer field value character");
  }

  match data[consumed..] {
    double_crlf!() => {
      callback!(on_trailer_value, consumed);
      callback!(on_trailers);
      complete_message(parser, (consumed + 4) as isize)
    }
    crlf!() => {
      callback!(on_trailer_value, consumed);
      move_to!(trailer_name, consumed + 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid trailer field value character"),
    _ => suspend!(),
  }
});

// #endregion trailers

generate_parser!();

impl Parser {
  pub fn new() -> Parser {
    Parser {
      owner: None,
      paused: false,
      state: initial_state!(),
      position: 0,
      values: Values::new(),
      spans: Spans::new(),
      callbacks: Callbacks::new(),
      error_code: Error::NONE,
      error_description: Vec::with_capacity(1024),
    }
  }

  pub fn reset(&mut self, keep_position: bool) {
    self.state = initial_state!();
    self.paused = false;

    if !keep_position {
      self.position = 0;
    }
    self.spans.unconsumed.clear();
    self.spans.body.clear();
    self.values.clear();
    self.error_code = Error::NONE;
    self.error_description.clear();
  }

  pub fn parse(&mut self, data: *const c_uchar, mut limit: usize) -> usize {
    if self.paused {
      return 0;
    }

    let unconsumed_len = self.spans.unconsumed.len();
    let mut aggregate: Vec<c_uchar>;
    let mut consumed = 0;
    let additional = unsafe { slice::from_raw_parts(data, limit) };

    let mut current = if unconsumed_len > 0 {
      limit += unconsumed_len;
      let unconsumed = &self.spans.unconsumed[..];

      aggregate = vec![];
      aggregate.extend_from_slice(unconsumed);
      aggregate.extend_from_slice(additional);

      &aggregate[..]
    } else {
      additional
    };

    #[cfg(debug_assertions)]
    if self.position == 0 {
      if (self.callbacks.before_state_change)(self, ptr::null(), 0) > 0 {
        self.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
      }
    }

    current = &current[..limit];

    #[cfg(all(debug_assertions, feature = "milo_debug_loop"))]
    let mut last = Instant::now();

    #[cfg(all(debug_assertions, feature = "milo_debug_loop"))]
    let mut initial_state = self.state;

    while current.len() > 0 || self.values.continue_without_data == 1 {
      self.values.continue_without_data = 0;

      // Since states might advance position manually, we have to explicitly track it
      let initial_position = self.position;

      if let State::FINISH = self.state {
        if self.values.continue_without_data == 0 {
          self.fail_str(Error::UNEXPECTED_DATA, "unexpected data");
          continue;
        }
      }

      let result = apply_state!();

      match &self.state {
        State::FINISH => {
          (self.callbacks.on_finish)(self, ptr::null(), 0);
        }
        State::ERROR => {
          (self.callbacks.on_error)(self, self.error_description.as_ptr(), self.error_description.len());
          break;
        }
        _ => {}
      }

      // Negative return values mean to consume N bytes and then pause.
      // Returning PAUSE from a callback instructs to pause without consuming any
      // byte.
      if result == SUSPEND {
        break;
      }

      let advance = result as usize;
      self.position += advance;

      let difference = self.position - initial_position;
      consumed += difference;
      current = &current[difference..];

      #[cfg(all(debug_assertions, feature = "milo_debug_loop"))]
      {
        let duration = Instant::now().duration_since(last).as_nanos();

        if duration > 100 {
          println!(
            "[milo::debug] loop iteration ({} -> {}) completed in {} ns",
            initial_state, self.state, duration
          );
        }

        last = Instant::now();
        initial_state = self.state;
      }

      // If a callback paused the parser, break now
      if self.paused {
        break;
      }
    }

    if consumed < limit {
      self.spans.unconsumed = current.to_vec();
    } else {
      self.spans.unconsumed.clear();
    }

    consumed
  }

  #[allow(dead_code)]
  fn move_to(&mut self, state: State, advance: isize) -> isize {
    #[cfg(debug_assertions)]
    {
      // Notify the end of the current state
      if (self.callbacks.before_state_change)(self, ptr::null(), 0) != 0 {
        return self.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
      }
    };

    // Change the state
    self.state = state;

    #[cfg(debug_assertions)]
    {
      // Notify the end of the current state
      if (self.callbacks.after_state_change)(self, ptr::null(), 0) != 0 {
        return self.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
      }
    };

    advance
  }

  fn fail(&mut self, code: Error, reason: String) -> isize {
    self.error_code = code;
    self.error_description = reason.as_bytes().to_vec();
    self.state = State::ERROR;

    0
  }

  fn fail_str(&mut self, code: Error, reason: &str) -> isize { self.fail(code, reason.into()) }

  pub fn pause(&mut self) { self.paused = true; }

  pub fn resume(&mut self) { self.paused = false; }

  pub fn finish(&mut self) {
    match self.state {
      State::START | State::REQUEST | State::RESPONSE | State::FINISH => {
        self.state = State::FINISH;
      }
      State::BODY_WITH_NO_LENGTH => {
        let action = (self.callbacks.on_message_complete)(self, ptr::null(), 0);

        if action != 0 {
          self.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
        }

        self.state = State::FINISH;
      }
      State::ERROR => (),
      _ => {
        self.fail_str(Error::UNEXPECTED_EOF, "Unexpected end of data");
      }
    };
  }
}

#[no_mangle]
pub extern "C" fn free_string(s: *const c_uchar) {
  unsafe {
    if s.is_null() {
      return;
    }

    let _ = CString::from_raw(s as *mut c_char);
  }
}

#[no_mangle]
pub extern "C" fn create_parser() -> *mut Parser { Box::into_raw(Box::new(Parser::new())) }

#[no_mangle]
pub extern "C" fn free_parser(ptr: *mut Parser) {
  if ptr.is_null() {
    return;
  }

  unsafe {
    let _ = Box::from_raw(ptr);
  }
}

#[no_mangle]
pub extern "C" fn reset_parser(parser: *mut Parser, keep_position: bool) {
  unsafe { parser.as_mut().unwrap().reset(keep_position) }
}

#[no_mangle]
pub extern "C" fn execute_parser(parser: *mut Parser, data: *const c_uchar, limit: usize) -> usize {
  unsafe { parser.as_mut().unwrap().parse(data, limit) }
}

#[no_mangle]
pub extern "C" fn pause_parser(parser: *mut Parser) { unsafe { parser.as_mut().unwrap().pause() } }

#[no_mangle]
pub extern "C" fn resume_parser(parser: *mut Parser) { unsafe { parser.as_mut().unwrap().resume() } }

#[no_mangle]
pub extern "C" fn finish_parser(parser: *mut Parser) { unsafe { parser.as_mut().unwrap().finish() } }

#[no_mangle]
pub extern "C" fn is_paused(parser: *mut Parser) -> bool { unsafe { parser.as_mut().unwrap().paused } }

#[no_mangle]
pub extern "C" fn get_owner(parser: *mut Parser) -> *mut c_void {
  unsafe {
    match parser.as_mut().unwrap().owner {
      Some(x) => x,
      None => ptr::null_mut(),
    }
  }
}

#[no_mangle]
pub extern "C" fn set_owner(parser: *mut Parser, ptr: *mut c_void) {
  unsafe {
    parser.as_mut().unwrap().owner = if ptr.is_null() { None } else { Some(ptr) };
  }
}

#[no_mangle]
pub extern "C" fn get_state(parser: *mut Parser) -> u8 { unsafe { parser.as_mut().unwrap().state as u8 } }

#[no_mangle]
pub extern "C" fn get_state_string(parser: *mut Parser) -> *const c_uchar {
  let string = c_match_state_string!();

  CString::new(string).unwrap().into_raw() as *const c_uchar
}

#[no_mangle]
pub extern "C" fn get_position(parser: *mut Parser) -> usize { unsafe { (*parser).position } }

#[no_mangle]
pub extern "C" fn get_error_code(parser: *mut Parser) -> u8 { unsafe { (*parser).error_code as u8 } }

#[no_mangle]
pub extern "C" fn get_error_code_string(parser: *mut Parser) -> *const c_uchar {
  let string = c_match_error_code_string!();

  CString::new(string).unwrap().into_raw() as *const c_uchar
}

#[no_mangle]
pub extern "C" fn get_error_description_string(parser: *mut Parser) -> *const c_uchar {
  unsafe { CString::from_vec_unchecked((*parser).error_description.clone()).into_raw() as *const c_uchar }
}

c_values_getters!();

c_values_setters!();

c_spans_getters!();

c_callbacks_setters!();

#[no_mangle]
pub extern "C" fn noop(_parser: &mut Parser, _data: *const c_uchar, _len: usize) -> isize { 0 }
