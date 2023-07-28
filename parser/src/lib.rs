#![feature(vec_into_raw_parts)]

#[macro_use]
extern crate lazy_static;

use std::ffi::CString;
use std::fmt::Debug;
use std::os::raw::{c_char, c_uchar, c_void};
use std::ptr;
use std::slice::from_raw_parts;
use std::str;
#[cfg(all(debug_assertions, feature = "milo_debug_loop"))]
use std::time::Instant;

use milo_parser_macros::{
  apply_state, c_match_error_code_string, c_match_state_string, callback, callbacks, case_insensitive_string, char,
  consume, crlf, digit, double_crlf, errors, fail, find_method, generate_parser_types, hex_digit, initial_state,
  method, move_to, otherwise, state, string, string_length, suspend, token, token_value, url,
};

/// cbindgen:ignore
pub mod test_utils;

pub const AUTODETECT: isize = 0;
pub const REQUEST: isize = 1;
pub const RESPONSE: isize = 2;

pub const CONNECTION_KEEPALIVE: isize = 0;
pub const CONNECTION_CLOSE: isize = 1;
pub const CONNECTION_UPGRADE: isize = 2;

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
  before_state_change,
  after_state_change,
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
  on_body,
  on_data,
  on_trailer_name,
  on_trailer_value,
  on_trailers
);

// TODO@PI: Add define_mixin! and use_mixin! macro to avoid fn calls - This can
// be used to eagerly process common headers

#[inline(always)]
fn store_parsed_http_version(parser: &mut Parser, major: c_uchar) {
  if major == char!('1') {
    parser.version_major = 1;
    parser.version_minor = 1;
  } else {
    parser.version_major = 2;
    parser.version_minor = 0;
  }
}

// #region general
// Depending on the mode flag, choose the initial state
state!(start, {
  match parser.mode {
    AUTODETECT => move_to!(message, 0),
    REQUEST => {
      parser.message_type = REQUEST;
      callback!(on_message_start);
      callback!(on_request);
      move_to!(request, 0)
    }
    RESPONSE => {
      parser.message_type = RESPONSE;
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
      parser.message_type = RESPONSE;
      callback!(on_message_start);
      callback!(on_response);
      move_to!(response, 0)
    }
    method!() => {
      parser.message_type = REQUEST;
      callback!(on_message_start);
      callback!(on_request);
      move_to!(request, 0)
    }
    otherwise!(5) => fail!(UNEXPECTED_CHARACTER, "Unexpected data"),
    _ => suspend!(),
  }
});

state!(end, {
  let must_close = parser.connection == CONNECTION_CLOSE;
  parser.connection = 0;

  if must_close {
    move_to!(finish, 0)
  } else {
    move_to!(start, 0)
  }
});
// #endregion general

// #region request - Request line parsing
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
      parser.method = method;

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

          // Reject HTTP/2.0
          if parser.method == METHOD_PRI {
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

// #region response - Status line
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
      // Store the status as integer
      parser.status = unsafe { str::from_utf8_unchecked(&data[0..3]).parse::<isize>().unwrap() };

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

// #region headers - Headers
// RFC 9112 section 4
state!(header_name, {
  // Special headers treating
  match data {
    case_insensitive_string!("content-length:") => {
      let status = parser.status;

      if parser.has_chunked_transfer_encoding == 1 {
        return fail!(
          UNEXPECTED_CONTENT_LENGTH,
          "Unexpected Content-Length header when Transfer-Encoding header is present"
        );
      } else if parser.status == 204 || parser.status / 100 == 1 {
        return fail!(
          UNEXPECTED_CONTENT_LENGTH,
          format!("Unexpected Content-Length header for a response with status {}", status)
        );
      } else if parser.content_length != 0 {
        return fail!(INVALID_CONTENT_LENGTH, "Invalid duplicate Content-Length header");
      }

      parser.has_content_length = 1;
      callback!(on_header_name, string_length!("content-length"));
      return move_to!(header_content_length, string_length!("content-length", 1));
    }
    case_insensitive_string!("transfer-encoding:") => {
      let status = parser.status;

      if parser.content_length > 0 {
        return fail!(
          UNEXPECTED_TRANSFER_ENCODING,
          "Unexpected Transfer-Encoding header when Content-Length header is present"
        );
      } else if parser.status == 304 {
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
      parser.has_trailers = 1;
      callback!(on_header_name, string_length!("trailer"));
      return move_to!(header_value, string_length!("trailer", 1));
    }
    // RFC 9110 section 7.8
    case_insensitive_string!("upgrade:") => {
      parser.has_upgrade = 1;
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
      parser.continue_without_data = 1;
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
    if parser.has_chunked_transfer_encoding == 1 {
      return fail!(
        INVALID_TRANSFER_ENCODING,
        "The value \"chunked\" in the Transfer-Encoding header must be the last provided and can be provided only once"
      );
    }

    parser.has_chunked_transfer_encoding = 1;
  } else if parser.has_chunked_transfer_encoding == 1 {
    // Any other value when chunked was already specified is invalid as the previous
    // chunked would not be the last one anymore
    return fail!(
      INVALID_TRANSFER_ENCODING,
      "The value \"chunked\" in the Transfer-Encoding header must be the last provided"
    );
  }

  consume!(token_value!());

  if consumed == 0 {
    return fail!(INVALID_TRANSFER_ENCODING, "Invalid header field value character");
  }

  match data[consumed..] {
    double_crlf!() => {
      callback!(on_header_value, consumed);
      parser.position += consumed;
      parser.continue_without_data = 1;
      move_to!(headers, 4)
    }
    crlf!() => {
      callback!(on_header_value, consumed);
      move_to!(header_name, consumed + 2)
    }
    otherwise!(2) => fail!(INVALID_TRANSFER_ENCODING, "Invalid header field value character"),
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
    return fail!(INVALID_CONTENT_LENGTH, "Invalid header field value character");
  }

  match data[consumed..] {
    crlf!() => {
      if let Ok(length) = unsafe { str::from_utf8_unchecked(&data[0..consumed]) }.parse::<usize>() {
        parser.content_length = length as isize;
        parser.remaining_content_length = parser.content_length;

        callback!(on_header_value, consumed);
        move_to!(header_name, consumed + 2)
      } else {
        fail!(INVALID_CONTENT_LENGTH, "Invalid Content-Length header")
      }
    }
    otherwise!(2) => fail!(INVALID_CONTENT_LENGTH, "Invalid header field value character"),
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
      parser.connection = CONNECTION_CLOSE;
      callback!(on_header_value, string_length!("close"));
      return move_to!(header_name, string_length!("close", 2));
    }
    case_insensitive_string!("keep-alive\r\n") => {
      parser.connection = CONNECTION_KEEPALIVE;
      callback!(on_header_value, string_length!("keep-alive"));
      return move_to!(header_name, string_length!("keep-alive", 2));
    }
    case_insensitive_string!("upgrade\r\n") => {
      parser.connection = CONNECTION_UPGRADE;
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
      parser.continue_without_data = 1;
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

  // Strip trailing OWS
  let mut trimmed_consumed = consumed;
  while let char!('\t') | char!(' ') = data[trimmed_consumed - 1] {
    trimmed_consumed -= 1;
  }

  match data[consumed..] {
    double_crlf!() => {
      callback!(on_header_value, trimmed_consumed);
      parser.position += consumed;
      parser.continue_without_data = 1;
      move_to!(headers, 4)
    }
    crlf!() => {
      callback!(on_header_value, trimmed_consumed);
      move_to!(header_name, consumed + 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid header field value character"),
    _ => suspend!(),
  }
});

// RFC 9110 section 9.3.6 and 7.8 - Headers have finished, check if the
// connection must be upgraded or a body is expected.
state!(headers, {
  parser.continue_without_data = 1;

  if parser.has_upgrade == 1 && parser.connection != CONNECTION_UPGRADE {
    parser.continue_without_data = 0;

    return fail!(
      MISSING_CONNECTION_UPGRADE,
      format!("Missing Connection header set to \"upgrade\" when using the Upgrade header")
    );
  }

  callback!(on_headers);

  let method = parser.method;
  let status = parser.status;

  // In case of Connection: Upgrade
  if parser.has_upgrade == 1 {
    if parser.connection != CONNECTION_UPGRADE {
      parser.continue_without_data = 0;

      return fail!(
        MISSING_CONNECTION_UPGRADE,
        format!("Missing Connection header set to \"upgrade\" when using the Upgrade header")
      );
    }

    callback!(on_upgrade);
    return move_to!(tunnel, 0);
  }

  // In case of CONNECT method
  if parser.is_connect_request == 1 {
    callback!(on_connect);
    return move_to!(tunnel, 0);
  }

  if (method == METHOD_GET || method == METHOD_HEAD) && parser.content_length > 0 {
    parser.continue_without_data = 0;

    return fail!(UNEXPECTED_CONTENT, format!("Unexpected content for {} request", method));
  }

  // RFC 9110 section 6.3
  if parser.message_type == REQUEST {
    if parser.has_content_length == 1 {
      if parser.content_length == 0 {
        return complete_message(parser, 0);
      }
    } else if parser.has_chunked_transfer_encoding == 0 {
      return complete_message(parser, 0);
    }
  } else {
    if (status < 200 && status != 101) || method == METHOD_HEAD || parser.skip_body == 1 {
      return complete_message(parser, 0);
    }

    if parser.content_length == 0 {
      if parser.has_content_length == 1 {
        return complete_message(parser, 0);
      } else if parser.has_chunked_transfer_encoding == 0 {
        return move_to!(body_with_no_length, 0);
      }
    }
  }

  move_to!(body, 0)
});

// #endregion headers

// RFC 9110 section 6.4.1 - Message completed
#[inline(always)]
fn complete_message(parser: &mut Parser, advance: isize) -> isize {
  callback!(on_message_complete);

  let connection = parser.connection;

  parser.clear();
  parser.continue_without_data = 1;
  parser.connection = connection;

  callback!(on_reset);
  move_to!(end, advance)
}

// #region common_body - Check if the body uses chunked encoding
state!(body, {
  if parser.content_length > 0 {
    return move_to!(body_via_content_length, 0);
  }

  if parser.has_trailers == 1 && !parser.has_chunked_transfer_encoding == 0 {
    return fail!(
      UNTRAILERS,
      "Trailers are not allowed when not using chunked transfer encoding"
    );
  }

  move_to!(chunk_length, 0)
});

// Return PAUSE makes this method idempotent without failing - In this state
// all data is ignored since the connection is not in HTTP anymore
state!(tunnel, { suspend!() });

// #endregion common_body

// #region body via Content-Length
// RFC 9112 section 6.2
state!(body_via_content_length, {
  let expected = parser.remaining_content_length as usize;
  let available = data.len();

  // Less data than what it is expected
  if available < expected {
    parser.remaining_content_length -= available as isize;
    callback!(on_data, available);

    return available as isize;
  }

  callback!(on_data, expected);
  callback!(on_body);
  complete_message(parser, expected as isize)
});
// #endregion body via Content-Length

// RFC 9110 section 6.3 - Body with no length nor chunked encoding. This is only
// allowed in responses.
//
// Note that on_body can't and will not be called here as there is no way to
// know when the response finishes.
state!(body_with_no_length, {
  let len = data.len();
  callback!(on_data, len);
  len as isize
});

// #region body via chunked Transfer-Encoding
// RFC 9112 section 7.1
state!(chunk_length, {
  consume!(hex_digit!());

  match data[consumed..] {
    [char!(';'), ..] if consumed > 0 => {
      // Parse the length as integer
      if let Ok(length) = usize::from_str_radix(unsafe { str::from_utf8_unchecked(&data[..consumed]) }, 16) {
        callback!(on_chunk_length, consumed);
        parser.chunk_size = length as isize;
        parser.remaining_chunk_size = parser.chunk_size;
        move_to!(chunk_extension_name, consumed + 1)
      } else {
        fail!(INVALID_CHUNK_SIZE, "Invalid chunk length")
      }
    }
    crlf!() => {
      if let Ok(length) = usize::from_str_radix(unsafe { str::from_utf8_unchecked(&data[..consumed]) }, 16) {
        // Parse the length as integer
        callback!(on_chunk_length, consumed);
        parser.chunk_size = length as isize;
        parser.remaining_chunk_size = parser.chunk_size;
        parser.continue_without_data = 1;
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

      parser.continue_without_data = 1;
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
      parser.continue_without_data = 1;
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
      parser.continue_without_data = 1;
      callback!(on_chunk_extension_value, consumed - 1);
      move_to!(chunk_data, consumed + 2)
    }
    [char!(';'), ..] => {
      parser.continue_without_data = 1;
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
  // When receiving the last chunk
  if parser.chunk_size == 0 {
    callback!(on_body);

    if parser.has_trailers == 1 {
      return move_to!(trailer_name, 0);
    } else {
      return move_to!(crlf_after_last_chunk, 0);
    }
  }

  let expected = parser.remaining_chunk_size as usize;
  let available = data.len();

  // Less data than what it is expected for this chunk
  if available < expected {
    parser.remaining_chunk_size -= available as isize;
    callback!(on_data, available);

    return available as isize;
  }

  callback!(on_data, expected);
  callback!(on_body);
  move_to!(chunk_end, expected)
});

state!(chunk_end, {
  match data {
    crlf!() => {
      parser.chunk_size = 0;
      parser.remaining_chunk_size = 0;
      move_to!(chunk_length, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Unexpected character after chunk data"),
    _ => suspend!(),
  }
});

state!(crlf_after_last_chunk, {
  match data {
    crlf!() => {
      parser.continue_without_data = 1;
      complete_message(parser, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Expected CRLF after the last chunk"),
    _ => suspend!(),
  }
});

// #endregion body via chunked Transfer-Encoding

// #region trailers - Trailers
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

generate_parser_types!();

#[repr(C)]
#[derive(Debug)]
pub struct Parser {
  pub owner: *mut c_void,
  pub state: State,
  pub position: usize,
  pub paused: bool,
  pub error_code: Error,
  pub error_description: *const c_uchar,
  pub error_description_len: usize,
  pub unconsumed: *const c_uchar,
  pub unconsumed_len: usize,
  pub id: isize,
  pub mode: isize,
  pub continue_without_data: isize,
  pub message_type: isize,
  pub is_connect_request: isize,
  pub method: isize,
  pub status: isize,
  pub version_major: isize,
  pub version_minor: isize,
  pub connection: isize,
  pub has_content_length: isize,
  pub has_chunked_transfer_encoding: isize,
  pub has_upgrade: isize,
  pub has_trailers: isize,
  pub content_length: isize,
  pub chunk_size: isize,
  pub remaining_content_length: isize,
  pub remaining_chunk_size: isize,
  pub skip_body: isize,
  pub callbacks: Callbacks,
}

impl Parser {
  /// Creates a new parser.
  pub fn new() -> Parser {
    Parser {
      owner: ptr::null_mut(),
      state: initial_state!(),
      position: 0,
      paused: false,
      error_code: Error::NONE,
      error_description: ptr::null(),
      error_description_len: 0,
      unconsumed: ptr::null(),
      unconsumed_len: 0,
      id: 0,
      mode: 0,
      continue_without_data: 0,
      message_type: 0,
      is_connect_request: 0,
      method: 0,
      status: 0,
      version_major: 0,
      version_minor: 0,
      connection: 0,
      has_content_length: 0,
      has_chunked_transfer_encoding: 0,
      has_upgrade: 0,
      has_trailers: 0,
      content_length: 0,
      chunk_size: 0,
      remaining_content_length: 0,
      remaining_chunk_size: 0,
      skip_body: 0,
      callbacks: Callbacks::new(),
    }
  }

  /// Resets a parser. The second parameters specifies if to also reset the
  /// position counter.
  pub fn reset(&mut self, keep_position: bool) {
    self.state = initial_state!();
    self.paused = false;

    if !keep_position {
      self.position = 0;
    }

    self.error_code = Error::NONE;

    if self.error_description_len > 0 {
      unsafe {
        Vec::from_raw_parts(
          self.error_description as *mut c_uchar,
          self.error_description_len,
          self.error_description_len,
        );
      }

      self.error_description = ptr::null();
      self.error_description_len = 0;
    }

    if self.unconsumed_len > 0 {
      unsafe {
        Vec::from_raw_parts(
          self.unconsumed as *mut c_uchar,
          self.unconsumed_len,
          self.unconsumed_len,
        );
      }

      self.unconsumed = ptr::null();
      self.unconsumed_len = 0;
    }

    self.clear();

    let on_reset_result = (self.callbacks.on_reset)(self, ptr::null(), 0);
    if on_reset_result != 0 {
      self.fail(
        Error::CALLBACK_ERROR,
        format!("Callback on_reset failed with return value {}.", on_reset_result),
      );
    }
  }

  /// Clears all values in the parser.
  ///
  /// Persisted fields, unconsumed data and the position are not cleared.
  pub fn clear(&mut self) {
    self.message_type = 0;
    self.is_connect_request = 0;
    self.method = 0;
    self.status = 0;
    self.version_major = 0;
    self.version_minor = 0;
    self.connection = 0;
    self.has_content_length = 0;
    self.has_chunked_transfer_encoding = 0;
    self.has_upgrade = 0;
    self.has_trailers = 0;
    self.content_length = 0;
    self.chunk_size = 0;
    self.remaining_content_length = 0;
    self.remaining_chunk_size = 0;
    self.skip_body = 0;
  }

  /// # Safety
  ///
  /// Parses a slice of characters. It returns the number of consumed
  /// characters.
  pub unsafe fn parse(&mut self, data: *const c_uchar, mut limit: usize) -> usize {
    // If the parser is paused, this is a no-op
    if self.paused {
      return 0;
    }

    let aggregate: Vec<c_uchar>;
    let mut consumed = 0;
    let additional = unsafe { from_raw_parts(data, limit) };

    // Set the data to analyze, prepending unconsumed data from previous iteration
    // if needed
    let mut current = if self.unconsumed_len > 0 {
      unsafe {
        limit += self.unconsumed_len;
        let unconsumed = from_raw_parts(self.unconsumed, self.unconsumed_len);

        aggregate = [unconsumed, additional].concat();
        &aggregate[..]
      }
    } else {
      additional
    };

    // Notify initial state change when starting
    #[cfg(debug_assertions)]
    if self.position == 0 {
      let on_before_state_change_result = (self.callbacks.before_state_change)(self, ptr::null(), 0);
      if on_before_state_change_result != 0 {
        self.fail(
          Error::CALLBACK_ERROR,
          format!(
            "Callback before_state_change failed with return value {}.",
            on_before_state_change_result,
          ),
        );
      }
    }

    // Limit the data that is currently analyzed
    current = &current[..limit];

    #[cfg(all(debug_assertions, feature = "milo_debug_loop"))]
    let mut last = Instant::now();

    #[cfg(all(debug_assertions, feature = "milo_debug_loop"))]
    let mut initial_state = self.state;

    // Until there is data or there is a request to continue
    while !current.is_empty() || self.continue_without_data == 1 {
      // Reset the continue_without_data bit
      self.continue_without_data = 0;

      // Since states might advance position manually, the parser have to explicitly
      // track it
      let initial_position = self.position;

      // If the parser has finished and it receives more data, error
      if let State::FINISH = self.state {
        if self.continue_without_data == 0 {
          self.fail_str(Error::UNEXPECTED_DATA, "unexpected data");
          continue;
        }
      }

      // Apply the current state
      let result = apply_state!();

      // If the parser finished or errored, execute callbacks
      match &self.state {
        State::FINISH => {
          (self.callbacks.on_finish)(self, ptr::null(), 0);
        }
        State::ERROR => {
          (self.callbacks.on_error)(self, self.error_description, self.error_description_len);
          break;
        }
        _ => {}
      }

      // If the state suspended the parser, then bail out earlier
      if result == SUSPEND {
        break;
      }

      // Update the position of the parser
      let advance = result as usize;
      self.position += advance;

      // Compute how many bytes were actually consumed and then advance the data
      let difference = self.position - initial_position;
      consumed += difference;
      current = &current[difference..];

      // Show the duration of the operation if asked to
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

    unsafe {
      // Drop any previous retained data
      if self.unconsumed_len > 0 {
        Vec::from_raw_parts(
          self.unconsumed as *mut c_uchar,
          self.unconsumed_len,
          self.unconsumed_len,
        );

        self.unconsumed = ptr::null();
        self.unconsumed_len = 0;
      }

      // If less bytes were consumed than requested, copy the unconsumed portion in
      // the parser for the next iteration
      if consumed < limit {
        let (ptr, len, _) = current.to_vec().into_raw_parts();

        self.unconsumed = ptr;
        self.unconsumed_len = len;
      }
    }

    // Return the number of consumed bytes
    consumed
  }

  /// Moves the parsers to a new state and marks a certain number of characters
  /// as used.
  ///
  /// The allow annotation is needed when building in release mode.
  #[allow(dead_code)]
  fn move_to(&mut self, state: State, advance: isize) -> isize {
    // Notify the end of the current state
    #[cfg(debug_assertions)]
    {
      let on_before_state_change_result = (self.callbacks.after_state_change)(self, ptr::null(), 0);
      if (self.callbacks.before_state_change)(self, ptr::null(), 0) != 0 {
        return self.fail(
          Error::CALLBACK_ERROR,
          format!(
            "Callback before_state_change failed with return value {}.",
            on_before_state_change_result,
          ),
        );
      }
    };

    // Change the state
    self.state = state;

    // Notify the end of the current state
    #[cfg(debug_assertions)]
    {
      let on_after_state_change_result = (self.callbacks.after_state_change)(self, ptr::null(), 0);
      if on_after_state_change_result != 0 {
        return self.fail(
          Error::CALLBACK_ERROR,
          format!(
            "Callback after_state_change failed with return value {}.",
            on_after_state_change_result,
          ),
        );
      };
    }

    advance
  }

  /// Marks the parsing a failed, setting a error code and and error message.
  fn fail(&mut self, code: Error, reason: String) -> isize {
    // Set the code
    self.error_code = code;

    // Set the message
    let (ptr, len, _) = Vec::into_raw_parts(reason.as_bytes().into());

    self.error_description = ptr;
    self.error_description_len = len;
    self.state = State::ERROR;

    // Do not process any additional data
    0
  }

  /// Marks the parsing a failed, setting a error code and and error message.
  fn fail_str(&mut self, code: Error, reason: &str) -> isize { self.fail(code, reason.into()) }

  /// Pauses the parser. It will have to be resumed via `milo_resume`.
  pub fn pause(&mut self) { self.paused = true; }

  /// Resumes the parser.
  pub fn resume(&mut self) { self.paused = false; }

  /// Marks the parser as finished. Any new data received via `parse` will
  /// put the parser in the error state.
  pub fn finish(&mut self) {
    match self.state {
      // If the parser is one of the initial states, simply jump to finish
      State::START | State::REQUEST | State::RESPONSE | State::FINISH => {
        self.state = State::FINISH;
      }
      State::BODY_WITH_NO_LENGTH => {
        // Notify that the message has been completed
        let on_complete_result = (self.callbacks.on_message_complete)(self, ptr::null(), 0);
        if on_complete_result != 0 {
          self.fail(
            Error::CALLBACK_ERROR,
            format!("Callback on_complete failed with return value {}.", on_complete_result,),
          );
        }

        // Set the state to be finished
        self.state = State::FINISH;
      }
      State::ERROR => (),
      // In another other state, this is an error
      _ => {
        self.fail_str(Error::UNEXPECTED_EOF, "Unexpected end of data");
      }
    };
  }

  /// Returns the current parser's state as string.
  pub fn state_string(&self) -> String { c_match_state_string!() }

  /// Returns the current parser's error state as string.
  pub fn error_code_string(&self) -> String { c_match_error_code_string!() }

  /// Returns the current parser's error descrition.
  pub fn error_description_string(&self) -> String {
    unsafe { String::from_utf8_unchecked(from_raw_parts(self.error_description, self.error_description_len).to_vec()) }
  }
}

/// A callback that simply returns `0`.
///
/// Use this callback as pointer when you want to remove a callback from the
/// parser.
#[no_mangle]
pub extern "C" fn milo_noop(_parser: &mut Parser, _data: *const c_uchar, _len: usize) -> isize { 0 }

/// Cleans up memory used by a string previously returned by one of the milo's C
/// public interface.
#[no_mangle]
pub extern "C" fn milo_free_string(s: *const c_uchar) {
  unsafe {
    if s.is_null() {
      return;
    }

    let _ = CString::from_raw(s as *mut c_char);
  }
}

/// Creates a new parser.
#[no_mangle]
pub extern "C" fn milo_create() -> *mut Parser { Box::into_raw(Box::new(Parser::new())) }

/// # Safety
///
/// Destroys a parser.
#[no_mangle]
pub unsafe extern "C" fn milo_destroy(ptr: *mut Parser) {
  if ptr.is_null() {
    return;
  }

  unsafe {
    let _ = Box::from_raw(ptr);
  }
}

/// # Safety
///
/// Resets a parser to its initial state.
#[no_mangle]
pub unsafe extern "C" fn milo_reset(parser: *mut Parser, keep_position: bool) {
  unsafe { parser.as_mut().unwrap().reset(keep_position) }
}

/// # Safety
///
/// Parses a slice of characters. It returns the number of consumed characters.
#[no_mangle]
pub unsafe extern "C" fn milo_parse(parser: *mut Parser, data: *const c_uchar, limit: usize) -> usize {
  unsafe { parser.as_mut().unwrap().parse(data, limit) }
}

/// # Safety
///
/// Pauses the parser. It will have to be resumed via `milo_resume`.
#[no_mangle]
pub unsafe extern "C" fn milo_pause(parser: *mut Parser) { unsafe { parser.as_mut().unwrap().pause() } }

/// # Safety
///
/// Resumes the parser.
#[no_mangle]
pub unsafe extern "C" fn milo_resume(parser: *mut Parser) { unsafe { parser.as_mut().unwrap().resume() } }

/// # Safety
///
/// Marks the parser as finished. Any new data received via `milo_parse` will
/// put the parser in the error state.
#[no_mangle]
pub unsafe extern "C" fn milo_finish(parser: *mut Parser) { unsafe { parser.as_mut().unwrap().finish() } }

/// # Safety
///
/// Returns the current parser's state as string.
///
/// The returned value must be freed using `free_string`.
#[no_mangle]
pub unsafe extern "C" fn milo_state_string(parser: *mut Parser) -> *const c_uchar {
  unsafe {
    let value = parser.as_mut().unwrap().state_string();
    CString::new(value).unwrap().into_raw() as *const c_uchar
  }
}

/// # Safety
///
/// Returns the current parser's error state as string.
///
/// The returned value must be freed using `free_string`.
#[no_mangle]
pub unsafe extern "C" fn milo_error_code_string(parser: *mut Parser) -> *const c_uchar {
  unsafe {
    let value = parser.as_mut().unwrap().error_code_string();
    CString::new(value).unwrap().into_raw() as *const c_uchar
  }
}

/// # Safety
///
/// Returns the current parser's error descrition.
///
/// The returned value must be freed using `free_string`.
#[no_mangle]
pub unsafe extern "C" fn milo_error_description_string(parser: *mut Parser) -> *const c_uchar {
  unsafe {
    let value = parser.as_mut().unwrap().error_description_string();
    CString::new(value).unwrap().into_raw() as *const c_uchar
  }
}
