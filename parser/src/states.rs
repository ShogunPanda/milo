use alloc::vec::Vec;
use alloc::{boxed::Box, format};
use core::cell::{Cell, RefCell};
use core::ffi::{c_char, c_uchar, c_void};
use core::fmt::Debug;
use core::ptr;
use core::str;
use core::{slice, slice::from_raw_parts};
#[cfg(all(debug_assertions, feature = "debug"))]
use std::time::Instant;

use milo_macros::{
  advance, callback, case_insensitive_string, char, consume, crlf, digit, double_crlf, fail, find_method, method,
  move_to, next, otherwise, state, string, string_length, suspend, token,
};

use crate::*;

// Depending on the mode flag, choose the initial state
state!(start, {
  match self.mode {
    MESSAGE_TYPE_AUTODETECT => {
      move_to!(autodetect, 0);
    }
    MESSAGE_TYPE_REQUEST => {
      self.message_type = MESSAGE_TYPE_REQUEST;
      callback!(on_message_start);
      move_to!(request, 0);
    }
    MESSAGE_TYPE_RESPONSE => {
      self.message_type = MESSAGE_TYPE_RESPONSE;
      callback!(on_message_start);
      move_to!(response, 0);
    }
    _ => {
      fail!(UNEXPECTED_CHARACTER, "Invalid mode");
    }
  }
});

// If the parser has finished and it receives more data, error
state!(finish, {
  fail!(UNEXPECTED_CHARACTER, "Unexpected data");
});

// The error state is a no-op
state!(error, {
  suspend!();
});

// Autodetect if there is a HTTP/RTSP method or a response
state!(autodetect, {
  match data {
    crlf!() => {
      // RFC 9112 section 2.2
      advance!(2);
    }
    string!("HTTP/") | string!("RTSP/") => {
      self.message_type = MESSAGE_TYPE_RESPONSE;
      callback!(on_message_start);
      move_to!(response, 0);
    }
    method!() => {
      self.message_type = MESSAGE_TYPE_REQUEST;
      callback!(on_message_start);
      move_to!(request, 0);
    }
    otherwise!(5) => {
      fail!(UNEXPECTED_CHARACTER, "Unexpected data");
    }
    _ => {
      suspend!();
    }
  }
});
// #endregion general

// #region request - Request line parsing
// RFC 9112 section 3
state!(request, {
  match data {
    // RFC 9112 section 2.2 - Repeated
    crlf!() => {
      advance!(2);
    }
    [token!(), ..] => {
      self.clear();
      move_to!(request_method, 0);
    }
    otherwise!(2) => {
      fail!(UNEXPECTED_CHARACTER, "Expected method");
    }
    _ => {
      suspend!();
    }
  }
});

// RFC 9112 section 3.1
state!(request_method, {
  consume!(token);

  match data[consumed] {
    char!(' ') if consumed > 0 => {
      find_method!(&data[..consumed]);
      callback!(on_method, consumed);
      move_to!(request_url, consumed + 1);
    }
    _ => {
      fail!(UNEXPECTED_CHARACTER, "Expected token character");
    }
  }
});

// RFC 9112 section 3.2
state!(request_url, {
  consume!(url);

  match data[consumed] {
    char!(' ') if consumed > 0 => {
      callback!(on_url, consumed);
      move_to!(request_protocol, consumed + 1);
    }
    _ => {
      fail!(UNEXPECTED_CHARACTER, "Expected URL character");
    }
  }
});

// RFC 9112 section 2.3
state!(request_protocol, {
  match data {
    string!("HTTP/") | string!("RTSP/") => {
      callback!(on_protocol, 4);
      self.position += 4;

      move_to!(request_version, 1);
    }
    otherwise!(5) => {
      fail!(UNEXPECTED_CHARACTER, "Expected protocol");
    }
    _ => {
      suspend!();
    }
  }
});

state!(request_version, {
  match data {
    [digit!(), char!('.'), digit!(), char!('\r'), char!('\n'), ..] => {
      // Validate the version
      let version = &data[0..3];

      match version {
        string!("1.1") | string!("2.0") => {
          if data[0] == char!('1') {
            self.version_major = 1;
            self.version_minor = 1;
          } else {
            self.version_major = 2;
            self.version_minor = 0;
          }
          // Reject HTTP/2.0
          if self.method == METHOD_PRI {
            fail!(UNSUPPORTED_HTTP_VERSION, "HTTP/2.0 is not supported");
          }

          callback!(on_version, 3);

          self.position += 5;
          callback!(on_request);
          move_to!(header_name, 0);
        }
        _ => {
          fail!(INVALID_VERSION, "Invalid HTTP version");
        }
      }
    }
    otherwise!(5) => {
      fail!(UNEXPECTED_CHARACTER, "Expected HTTP version");
    }
    _ => {
      suspend!();
    }
  }
});
// #endregion request

// #region response - Status line
// RFC 9112 section 4
state!(response, {
  match data {
    crlf!() => {
      advance!(2);
    } // RFC 9112 section 2.2 - Repeated
    string!("HTTP/") | string!("RTSP/") => {
      self.clear();
      callback!(on_protocol, 4);
      move_to!(response_version, 5);
    }
    otherwise!(5) => {
      fail!(UNEXPECTED_CHARACTER, "Expected protocol");
    }
    _ => {
      suspend!();
    }
  }
});

state!(response_version, {
  match data {
    [digit!(), char!('.'), digit!(), char!(' '), ..] => {
      // Validate the version
      let version = &data[0..3];

      match version {
        string!("1.1") | string!("2.0") => {
          if data[0] == char!('1') {
            self.version_major = 1;
            self.version_minor = 1;
          } else {
            self.version_major = 2;
            self.version_minor = 0;
          }

          callback!(on_version, 3);
          move_to!(response_status, 4);
        }
        _ => {
          fail!(INVALID_VERSION, "Invalid HTTP version");
        }
      }
    }
    otherwise!(4) => {
      fail!(UNEXPECTED_CHARACTER, "Expected HTTP version");
    }
    _ => {
      suspend!();
    }
  }
});

state!(response_status, {
  // Collect the three digits
  match data {
    [digit!(), digit!(), digit!(), char!(' '), ..] => {
      // Store the status as integer
      self.status = unsafe { str::from_utf8_unchecked(&data[0..3]).parse::<usize>().unwrap() };
      callback!(on_status, 3);
      move_to!(response_reason, 4);
    }
    otherwise!(4) => {
      fail!(INVALID_STATUS, "Expected HTTP response status");
    }
    _ => {
      suspend!();
    }
  }
});

state!(response_reason, {
  consume!(token_value);

  match data[consumed..] {
    crlf!() => {
      if consumed > 0 {
        callback!(on_reason, consumed);
        self.position += consumed;
      }

      self.position += 2;
      callback!(on_response);
      move_to!(header_name, 0);
    }
    otherwise!(2) => {
      fail!(UNEXPECTED_CHARACTER, "Expected status reason");
    }
    _ => {
      suspend!();
    }
  }
});
// #endregion response

// #region headers - Headers
// RFC 9112 section.4
state!(header_name, {
  // Special headers treating
  match data {
    case_insensitive_string!("content-length:") => {
      let status = self.status;

      if self.has_chunked_transfer_encoding {
        fail!(
          UNEXPECTED_CONTENT_LENGTH,
          "Unexpected Content-Length header when Transfer-Encoding header is present"
        );
      } else if status == 204 || status / 100 == 1 {
        fail!(
          UNEXPECTED_CONTENT_LENGTH,
          "Unexpected Content-Length header for a response with status 204 or 1xx"
        );
      } else if self.content_length != 0 {
        fail!(INVALID_CONTENT_LENGTH, "Invalid duplicate Content-Length header");
      }

      self.has_content_length = true;
      callback!(on_header_name, string_length!("content-length"));
      move_to!(header_content_length, string_length!("content-length", 1));
      next!();
    }
    case_insensitive_string!("transfer-encoding:") => {
      let status = self.status;

      if self.content_length > 0 {
        fail!(
          UNEXPECTED_TRANSFER_ENCODING,
          "Unexpected Transfer-Encoding header when Content-Length header is present"
        );
      } else if status == 304 {
        // Transfer-Encoding is NOT allowed in 3.4
        fail!(
          UNEXPECTED_TRANSFER_ENCODING,
          "Unexpected Transfer-Encoding header for a response with status 304"
        );
      }

      callback!(on_header_name, string_length!("transfer-encoding"));
      move_to!(header_transfer_encoding, string_length!("transfer-encoding", 1));
      next!();
    }
    case_insensitive_string!("connection:") => {
      callback!(on_header_name, string_length!("connection"));
      move_to!(header_connection, string_length!("connection", 1));
      next!();
    }
    // RFC 9110 section 9.5
    case_insensitive_string!("trailer:") => {
      self.has_trailers = true;
      callback!(on_header_name, string_length!("trailer"));
      move_to!(header_value, string_length!("trailer", 1));
      next!();
    }
    // RFC 9110 section 7.8
    case_insensitive_string!("upgrade:") => {
      self.has_upgrade = true;
      callback!(on_header_name, string_length!("upgrade"));
      move_to!(header_value, string_length!("upgrade", 1));
      next!();
    }
    _ => {}
  }

  consume!(token);

  match data[consumed..] {
    [char!(':'), ..] if consumed > 0 => {
      callback!(on_header_name, consumed);
      move_to!(header_value, consumed + 1);
    }
    crlf!() => {
      self.continue_without_data = true;
      move_to!(headers, 2);
    }
    otherwise!(2) => {
      fail!(UNEXPECTED_CHARACTER, "Invalid header field name character");
    }
    _ => {
      suspend!();
    }
  }
});

// RFC 9112 section 6.1
state!(header_transfer_encoding, {
  // Ignore trailing OWS
  consume!(ws);

  if consumed > 0 {
    advance!(consumed);
    next!();
  }

  if let case_insensitive_string!("chunked\r\n")
  | case_insensitive_string!(",chunked\r\n")
  | case_insensitive_string!(", chunked\r\n") = data
  {
    // If this is 1, it means the Transfer-Encoding header was specified more than
    // once. This is the second repetition and therefore, the previous one is no
    // longer the last one, making it invalid.
    if self.has_chunked_transfer_encoding {
      fail!(
        INVALID_TRANSFER_ENCODING,
        "The value \"chunked\" in the Transfer-Encoding header must be the last provided and can be provided only once"
      );
    }

    self.has_chunked_transfer_encoding = true;
  } else if self.has_chunked_transfer_encoding {
    // Any other value when chunked was already specified is invalid as the previous
    // chunked would not be the last one anymore
    fail!(
      INVALID_TRANSFER_ENCODING,
      "The value \"chunked\" in the Transfer-Encoding header must be the last provided"
    );
  }

  consume!(token_value);

  if consumed == 0 {
    fail!(INVALID_TRANSFER_ENCODING, "Invalid header field value character");
  }

  match data[consumed..] {
    double_crlf!() => {
      callback!(on_header_value, consumed);
      self.position += consumed;
      self.continue_without_data = true;
      move_to!(headers, 4);
    }
    crlf!() => {
      callback!(on_header_value, consumed);
      move_to!(header_name, consumed + 2);
    }
    otherwise!(2) => {
      fail!(INVALID_TRANSFER_ENCODING, "Invalid header field value character");
    }
    _ => {
      suspend!();
    }
  }
});

// RFC 9112 section 6.2
state!(header_content_length, {
  // Ignore trailing OWS
  consume!(ws);

  if consumed > 0 {
    advance!(consumed);
    next!();
  }

  consume!(digit);

  if consumed == 0 {
    fail!(INVALID_CONTENT_LENGTH, "Invalid header field value character");
  }

  match data[consumed..] {
    crlf!() => {
      if let Ok(length) = unsafe { str::from_utf8_unchecked(&data[0..consumed]) }.parse::<u64>() {
        self.content_length = length;
        self.remaining_content_length = length;
        callback!(on_header_value, consumed);
        move_to!(header_name, consumed + 2);
      } else {
        fail!(INVALID_CONTENT_LENGTH, "Invalid Content-Length header");
      }
    }
    otherwise!(2) => {
      fail!(INVALID_CONTENT_LENGTH, "Invalid header field value character");
    }
    _ => {
      suspend!();
    }
  }
});

// RFC 9112 section 9.6
state!(header_connection, {
  // Ignore trailing OWS
  consume!(ws);

  if consumed > 0 {
    advance!(consumed);
    next!();
  }

  match data {
    case_insensitive_string!("close\r\n") => {
      self.connection = CONNECTION_CLOSE;
      callback!(on_header_value, string_length!("close"));
      move_to!(header_name, string_length!("close", 2));
      next!();
    }
    case_insensitive_string!("keep-alive\r\n") => {
      self.connection = CONNECTION_KEEPALIVE;
      callback!(on_header_value, string_length!("keep-alive"));
      move_to!(header_name, string_length!("keep-alive", 2));
      next!();
    }
    case_insensitive_string!("upgrade\r\n") => {
      self.connection = CONNECTION_UPGRADE;
      callback!(on_header_value, string_length!("upgrade"));
      move_to!(header_name, string_length!("upgrade", 2));
      next!();
    }
    _ => {}
  }

  consume!(token_value);

  if consumed == 0 {
    fail!(UNEXPECTED_CHARACTER, "Invalid header field value character");
  }

  match data[consumed..] {
    double_crlf!() => {
      callback!(on_header_value, consumed);
      self.position += consumed;
      self.continue_without_data = true;
      move_to!(headers, 4);
    }
    crlf!() => {
      callback!(on_header_value, consumed);
      move_to!(header_name, consumed + 2);
    }
    otherwise!(2) => {
      fail!(UNEXPECTED_CHARACTER, "Invalid header field value character");
    }
    _ => {
      suspend!();
    }
  }
});

// RFC 9110 section 5.5 and 5.6
state!(header_value, {
  // Ignore leading OWS
  consume!(ws);

  if consumed > 0 {
    advance!(consumed);
    next!();
  }

  consume!(token_value);

  let mut trimmed_consumed = 0;
  if consumed > 0 {
    // Strip trailing OWS
    trimmed_consumed = consumed;
    while let char!('\t') | char!(' ') = data[trimmed_consumed - 1] {
      trimmed_consumed -= 1;
    }
  }

  match data[consumed..] {
    double_crlf!() => {
      callback!(on_header_value, trimmed_consumed);
      self.position += consumed;
      self.continue_without_data = true;
      move_to!(headers, 4);
    }
    crlf!() => {
      callback!(on_header_value, trimmed_consumed);
      move_to!(header_name, consumed + 2);
    }
    otherwise!(2) => {
      fail!(UNEXPECTED_CHARACTER, "Invalid header field value character");
    }
    _ => {
      suspend!();
    }
  }
});

// RFC 9110 section 9.3.6 and 7.8 - Headers have finished, check if the
// connection must be upgraded or a body is expected.
state!(headers, {
  if self.has_upgrade && self.connection != CONNECTION_UPGRADE {
    fail!(
      MISSING_CONNECTION_UPGRADE,
      "Missing Connection header set to \"upgrade\" when using the Upgrade header"
    );
  }

  callback!(on_headers);

  let method = self.method;
  let status = self.status;

  // In case of Connection: Upgrade
  if self.has_upgrade {
    if self.connection != CONNECTION_UPGRADE {
      fail!(
        MISSING_CONNECTION_UPGRADE,
        "Missing Connection header set to \"upgrade\" when using the Upgrade header"
      );
    }

    callback!(on_upgrade);
    move_to!(tunnel, 0);
    next!();
  }

  // In case of CONNECT method
  if self.is_connect {
    callback!(on_connect);
    move_to!(tunnel, 0);
    next!();
  }

  if (method == METHOD_GET || method == METHOD_HEAD) && self.content_length > 0 {
    fail!(UNEXPECTED_CONTENT, "Unexpected content for the request (GET or HEAD)");
  }

  // RFC 9110 section 6.3
  if self.message_type == MESSAGE_TYPE_REQUEST {
    if self.has_content_length {
      if self.content_length == 0 {
        self.continue_without_data = true;
        move_to!(complete, 0);
        next!();
      }
    } else if !self.has_chunked_transfer_encoding {
      self.continue_without_data = true;
      move_to!(complete, 0);
      next!();
    }
  } else {
    if (status < 200 && status != 101) || method == METHOD_HEAD || self.skip_body {
      self.continue_without_data = true;
      move_to!(complete, 0);
      next!();
    }

    if self.content_length == 0 {
      if self.has_content_length {
        self.continue_without_data = true;
        move_to!(complete, 0);
        next!();
      } else if !self.has_chunked_transfer_encoding {
        move_to!(body_with_no_length, 0);
        next!();
      }
    } else if status == 304 {
      // RFC 9110 section 15.4.5
      self.continue_without_data = true;
      move_to!(complete, 0);
      next!();
    }
  }

  if self.content_length > 0 {
    move_to!(body_via_content_length, 0);
    next!();
  }

  if self.has_trailers && !self.has_chunked_transfer_encoding {
    fail!(
      UNEXPECTED_TRAILERS,
      "Trailers are not allowed when not using chunked transfer encoding"
    );
  }

  move_to!(chunk_length, 0);
});

// #endregion headers

// RFC 9110 section 6.4.1 - Message completed
state!(complete, {
  callback!(on_message_complete);
  callback!(on_reset);

  let must_close = self.connection == CONNECTION_CLOSE;

  if must_close {
    callback!(on_finish);
    move_to!(finish, 0);
  } else {
    move_to!(start, 0);
  }
});

// Return PAUSE makes this method idempotent without failing - In this state
// all data is ignored since the connection is not in HTTP anymore
state!(tunnel, {
  suspend!();
});

// #region body via Content-Length
// RFC 9112 section 6.2
state!(body_via_content_length, {
  let expected = self.remaining_content_length;
  let available_64 = available as u64;

  // Less data than what it is expected
  if available_64 < expected {
    self.remaining_content_length -= available_64;
    callback!(on_data, available);

    advance!(available);
    next!();
  }

  callback!(on_data, expected as usize);
  self.remaining_content_length = 0;
  callback!(on_body);
  self.continue_without_data = true;
  move_to!(complete, expected as usize);
});
// #endregion body via Content-Length

// RFC 9110 section 6.3 - Body with no length nor chunked encoding. This is only
// allowed in responses.
//
// Note that on_body can't and will not be called here as there is no way to
// know when the response finishes.
state!(body_with_no_length, {
  callback!(on_data, available);
  advance!(available);
});

// #region body via chunked Transfer-Encoding
// RFC 9112 section 7.1
state!(chunk_length, {
  consume!(hex_digit);

  match data[consumed..] {
    [char!(';'), ..] if consumed > 0 => {
      // Parse the length as integer
      if let Ok(length) = u64::from_str_radix(unsafe { str::from_utf8_unchecked(&data[..consumed]) }, 16) {
        callback!(on_chunk_length, consumed);
        self.chunk_size = length;
        self.remaining_chunk_size = length;
        move_to!(chunk_extension_name, consumed + 1);
      } else {
        fail!(INVALID_CHUNK_SIZE, "Invalid chunk length");
      }
    }
    crlf!() => {
      if let Ok(length) = u64::from_str_radix(unsafe { str::from_utf8_unchecked(&data[..consumed]) }, 16) {
        // Parse the length as integer
        callback!(on_chunk_length, consumed);
        self.chunk_size = length;
        self.remaining_chunk_size = length;
        self.continue_without_data = true;
        move_to!(chunk_data, consumed + 2);
      } else {
        fail!(INVALID_CHUNK_SIZE, "Invalid chunk length");
      }
    }
    otherwise!(2) => {
      fail!(UNEXPECTED_CHARACTER, "Invalid chunk length character");
    }
    _ => {
      suspend!();
    }
  }
});

state!(chunk_extension_name, {
  consume!(token);

  if consumed == 0 {
    fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension name character");
  }

  match data[consumed..] {
    [char!('='), ..] => {
      callback!(on_chunk_extension_name, consumed);
      move_to!(chunk_extension_value, consumed + 1);
    }
    [char!(';'), ..] => {
      callback!(on_chunk_extension_name, consumed);
      move_to!(chunk_extension_name, consumed + 1);
    }
    crlf!() => {
      callback!(on_chunk_extension_name, consumed);

      self.continue_without_data = true;
      move_to!(chunk_data, consumed + 2);
    }
    otherwise!(2) => {
      fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension name character");
    }
    _ => {
      suspend!();
    }
  }
});

state!(chunk_extension_value, {
  if data[0] == char!('"') {
    move_to!(chunk_extension_quoted_value, 1);
    next!();
  }

  consume!(token);

  if consumed == 0 {
    fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension value character");
  }

  match data[consumed..] {
    [char!(';'), ..] => {
      callback!(on_chunk_extension_value, consumed);
      move_to!(chunk_extension_name, consumed + 1);
    }
    crlf!() => {
      callback!(on_chunk_extension_value, consumed);
      self.continue_without_data = true;
      move_to!(chunk_data, consumed + 2);
    }
    otherwise!(2) => {
      fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension value character");
    }
    _ => {
      suspend!();
    }
  }
});

// RFC 9110 section 5.6.4
state!(chunk_extension_quoted_value, {
  // Also consume 0x22 and 0x5c as the quoted-pair validation is performed after
  consume!(token_value_quoted);

  if consumed == 0 || data[consumed - 1] != char!('"') {
    fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension quoted value");
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
    fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension quoted value");
  }

  match data[consumed..] {
    crlf!() => {
      self.continue_without_data = true;
      callback!(on_chunk_extension_value, consumed - 1);
      move_to!(chunk_data, consumed + 2);
    }
    [char!(';'), ..] => {
      self.continue_without_data = true;
      callback!(on_chunk_extension_value, consumed - 1);
      move_to!(chunk_extension_name, consumed + 1);
    }
    otherwise!(3) => {
      fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension quoted value character");
    }
    _ => {
      suspend!();
    }
  }
});

state!(chunk_data, {
  // When receiving the last chunk
  if self.chunk_size == 0 {
    callback!(on_chunk);
    callback!(on_body);

    if self.has_trailers {
      move_to!(trailer_name, 0);
    } else {
      move_to!(crlf_after_last_chunk, 0);
    }

    next!();
  }

  let expected = self.remaining_chunk_size;
  let available_64 = available as u64;

  // Less data than what it is expected for this chunk
  if available_64 < expected {
    self.remaining_chunk_size -= available_64;

    callback!(on_chunk);
    callback!(on_data, available);

    advance!(available);
    next!();
  }

  self.remaining_chunk_size = 0;

  callback!(on_chunk);
  callback!(on_data, expected as usize);

  move_to!(chunk_end, expected as usize);
});

state!(chunk_end, {
  match data {
    crlf!() => {
      self.chunk_size = 0;
      self.remaining_chunk_size = 0;
      move_to!(chunk_length, 2);
    }
    otherwise!(2) => {
      fail!(UNEXPECTED_CHARACTER, "Unexpected character after chunk data");
    }
    _ => {
      suspend!();
    }
  }
});

state!(crlf_after_last_chunk, {
  match data {
    crlf!() => {
      self.continue_without_data = true;
      move_to!(complete, 2);
    }
    otherwise!(2) => {
      fail!(UNEXPECTED_CHARACTER, "Expected CRLF after the last chunk");
    }
    _ => {
      suspend!();
    }
  }
});

// #endregion body via chunked Transfer-Encoding

// #region trailers - Trailers
// RFC 9112 section 7.1.2
state!(trailer_name, {
  consume!(token);

  match data[consumed..] {
    [char!(':'), ..] if consumed > 0 => {
      callback!(on_trailer_name, consumed);
      move_to!(trailer_value, consumed + 1);
    }
    crlf!() => {
      self.continue_without_data = true;
      move_to!(trailers, 2);
    }
    otherwise!(2) => {
      fail!(UNEXPECTED_CHARACTER, "Invalid trailer field name character");
    }
    _ => {
      suspend!();
    }
  }
});

state!(trailer_value, {
  // Ignore leading OWS
  consume!(ws);

  if consumed > 0 {
    advance!(consumed);
    next!();
  }

  consume!(token_value);

  let mut trimmed_consumed = 0;
  if consumed > 0 {
    // Strip trailing OWS
    trimmed_consumed = consumed;
    while let char!('\t') | char!(' ') = data[trimmed_consumed - 1] {
      trimmed_consumed -= 1;
    }
  }

  match data[consumed..] {
    double_crlf!() => {
      callback!(on_trailer_value, trimmed_consumed);
      self.position += consumed;
      self.continue_without_data = true;
      move_to!(trailers, 4);
    }
    crlf!() => {
      callback!(on_trailer_value, trimmed_consumed);
      move_to!(trailer_name, consumed + 2);
    }
    otherwise!(2) => {
      fail!(UNEXPECTED_CHARACTER, "Invalid trailer field value character");
    }
    _ => {
      suspend!();
    }
  }
});

state!(trailers, {
  callback!(on_trailers);
  self.continue_without_data = true;
  move_to!(complete, 0);
});
// #endregion trailers
