#[macro_use]
extern crate lazy_static;

use milo_parser_generator::{
  append, callback, callbacks, char, clear, crlf, data_slice_callback, digit, errors, fail, generate_parser,
  generate_parser_interface, get_span, hex_digit, method, move_to, otherwise, persistent_values, set_value, spans,
  state, string, suspend, token, url, user_writable_values, values,
};
use std::os::raw::c_char;

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
  skip_next_callback,
  message_type,
  is_connect_request,
  method,
  status,
  version_major,
  version_minor,
  connection,
  expected_content_length,
  expected_chunk_size,
  has_content_length,
  has_chunked_transfer_encoding,
  has_upgrade,
  has_trailers,
  current_content_length,
  current_chunk_size,
  skip_body
);

user_writable_values!(id, mode, is_connect_request, skip_body);

persistent_values!(id, mode, continue_without_data, skip_next_callback);

spans!(
  unconsumed,
  method,
  url,
  protocol,
  version,
  status,
  reason,
  header_field,
  header_value,
  body,
  chunk_length,
  chunk_extension_name,
  chunk_extension_value,
  chunk_data,
  trailer_field,
  trailer_value
);

errors!(
  NONE,
  UNEXPECTED_DATA,
  UNEXPECTED_EOF,
  CALLBACK_ERROR,
  UNEXPECTED_CHARACTER,
  UNEXPECTED_CONTENT_LENGTH,
  UNEXPECTED_TRANSFER_ENCODING,
  UNEXPECTED_CONTENT,
  UNEXPECTED_TRAILERS,
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
  on_method_complete,
  on_url,
  on_url_complete,
  on_protocol,
  on_protocol_complete,
  on_version,
  on_version_complete,
  on_status,
  on_status_complete,
  on_reason,
  on_reason_complete,
  on_header_field,
  on_header_field_complete,
  on_header_value,
  on_header_value_complete,
  on_headers_complete,
  on_connect,
  on_upgrade,
  on_chunk_length,
  on_chunk_extension_name,
  on_chunk_extension_value,
  on_chunk_data,
  on_body,
  on_trailer_field,
  on_trailer_value,
  on_trailers_complete
);

fn store_parsed_http_version(parser: &mut Parser, major: u8) {
  if major == b'1' {
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
    AUTODETECT => move_to!(message_start, 0),
    REQUEST => {
      parser.values.message_type = REQUEST;
      callback!(on_message_start);
      move_to!(request_start, 0)
    }
    RESPONSE => {
      parser.values.message_type = RESPONSE;
      callback!(on_message_start);
      move_to!(response_start, 0)
    }
    _ => fail!(UNEXPECTED_CHARACTER, "Invalid mode"),
  }
});

state!(finish, { 0 });

state!(error, { 0 });

// Autodetect if there is a HTTP/RTSP method or a response
state!(message_start, {
  match data {
    crlf!() => 2, // RFC 9112 section 2.2,
    method!() => {
      parser.values.message_type = REQUEST;
      callback!(on_message_start);
      callback!(on_request);
      move_to!(request_start, 0)
    }
    string!("HTTP/") | string!("RTSP/") => {
      parser.values.message_type = RESPONSE;
      callback!(on_message_start);
      callback!(on_response);
      move_to!(response_start, 0)
    }
    otherwise!(5) => fail!(UNEXPECTED_CHARACTER, "Unexpected data"),
    _ => suspend!(),
  }
});
// #general

// #region request
// RFC 9112 section 3
state!(request_start, {
  match data {
    crlf!() => 2, // RFC 9112 section 2.2 - Repeated
    token!(x) => {
      append!(method, x);
      move_to!(request_method)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Expected method"),
    _ => suspend!(),
  }
});

// RFC 9112 section 3.1
state!(request_method, {
  match data {
    token!(x) => {
      append!(method, x);
      1
    }
    char!(' ') => {
      // TODO@PI: Can we avoid CString here?
      let method_str = get_span!(method);
      let method = method_as_int(std::ffi::CString::new(method_str).unwrap().into_raw());

      if method == -1 {
        return fail!(UNEXPECTED_CHARACTER, "Invalid method");
      }

      if method == METHOD_CONNECT {
        parser.values.is_connect_request = 1;
      }

      parser.values.method = method;

      callback!(on_method, method);
      move_to!(request_method_complete)
    }
    _ => fail!(UNEXPECTED_CHARACTER, "Expected token character"),
  }
});

state!(request_method_complete, {
  callback!(on_method_complete);
  move_to!(request_url, 0)
});

// RFC 9112 section 3.2
state!(request_url, {
  match data {
    url!(x) => {
      append!(url, x);
      1
    }
    char!(' ') => {
      callback!(on_url, url);
      move_to!(request_url_complete)
    }
    _ => fail!(UNEXPECTED_CHARACTER, "Expected URL character"),
  }
});

state!(request_url_complete, {
  callback!(on_url_complete);
  move_to!(request_protocol, 0)
});

// RFC 9112 section 2.3
state!(request_protocol, {
  match data {
    [w @ b'H', x @ b'T', y @ b'T', z @ b'P', b'/', ..] => {
      append!(protocol, w);
      parser.position += 1;
      append!(protocol, x);
      parser.position += 1;
      append!(protocol, y);
      parser.position += 1;
      append!(protocol, z);
      parser.position += 1;
      callback!(on_protocol, protocol);
      move_to!(request_protocol_complete)
    }
    [w @ b'R', x @ b'T', y @ b'S', z @ b'P', b'/', ..] => {
      append!(protocol, w);
      parser.position += 1;
      append!(protocol, x);
      parser.position += 1;
      append!(protocol, y);
      parser.position += 1;
      append!(protocol, z);
      parser.position += 1;
      callback!(on_protocol, protocol);
      move_to!(request_protocol_complete)
    }
    otherwise!(5) => fail!(UNEXPECTED_CHARACTER, "Expected protocol"),
    _ => suspend!(),
  }
});

state!(request_protocol_complete, {
  callback!(on_protocol_complete);
  move_to!(request_version_major, 0)
});

state!(request_version_major, {
  match data {
    digit!(x) => {
      append!(version, x);
      1
    }
    [x @ b'.', ..] => {
      append!(version, x);
      move_to!(request_version_minor)
    }
    _ => parser.fail(
      Error::UNEXPECTED_CHARACTER,
      format!("Expected {} minor version", get_span!(protocol)),
    ),
  }
});

state!(request_version_minor, {
  match data {
    digit!(x) => {
      append!(version, x);
      1
    }
    crlf!() => {
      // Validate the version
      match parser.spans.version[..] {
        string!("1.1") | string!("2.0") => {
          callback!(on_version, version);
          store_parsed_http_version(parser, parser.spans.version[0]);
          move_to!(request_version_complete, 2)
        }
        _ => fail!(INVALID_VERSION, "Invalid HTTP version"),
      }
    }
    otherwise!(2) => parser.fail(
      Error::UNEXPECTED_CHARACTER,
      format!("Expected {} minor version", get_span!(protocol)),
    ),
    _ => suspend!(),
  }
});

state!(request_version_complete, {
  if parser.values.method == METHOD_PRI {
    return fail!(UNSUPPORTED_HTTP_VERSION, "HTTP/2.0 is not supported");
  }

  callback!(on_version_complete);
  move_to!(header_start, 0)
});
// #endregion request

// #region response
// RFC 9112 section 4
state!(response_start, {
  match data {
    crlf!() => 2, // RFC 9112 section 2.2 - Repeated
    [w @ b'H', x @ b'T', y @ b'T', z @ b'P', b'/', ..] => {
      append!(protocol, w);
      parser.position += 1;
      append!(protocol, x);
      parser.position += 1;
      append!(protocol, y);
      parser.position += 1;
      append!(protocol, z);
      parser.position += 1;
      callback!(on_protocol, protocol);
      move_to!(response_protocol_complete, 1)
    }
    [w @ b'R', x @ b'T', y @ b'S', z @ b'P', b'/', ..] => {
      append!(protocol, w);
      parser.position += 1;
      append!(protocol, x);
      parser.position += 1;
      append!(protocol, y);
      parser.position += 1;
      append!(protocol, z);
      parser.position += 1;
      callback!(on_protocol, protocol);
      move_to!(response_protocol_complete, 1)
    }
    otherwise!(5) => {
      fail!(UNEXPECTED_CHARACTER, "Expected protocol")
    }
    _ => suspend!(),
  }
});

state!(response_protocol_complete, {
  callback!(on_protocol_complete);
  move_to!(response_version_major, 0)
});

state!(response_version_major, {
  match data {
    digit!(x) => {
      append!(version, x);
      1
    }
    [x @ b'.', ..] => {
      append!(version, x);
      move_to!(response_version_minor)
    }
    _ => parser.fail(
      Error::UNEXPECTED_CHARACTER,
      format!(
        "Expected {} major version {}",
        unsafe { String::from_utf8_unchecked(parser.spans.protocol.clone()) },
        data[0]
      ),
    ),
  }
});

state!(response_version_minor, {
  match data {
    digit!(x) => {
      append!(version, x);
      1
    }
    char!(' ') => {
      // Validate the version
      match parser.spans.version[..] {
        string!("1.1") | string!("2.0") => {
          callback!(on_version, version);
          store_parsed_http_version(parser, parser.spans.version[0]);
          move_to!(response_version_complete)
        }
        _ => fail!(INVALID_VERSION, "Invalid HTTP version"),
      }
    }
    _ => parser.fail(
      Error::UNEXPECTED_CHARACTER,
      format!("Expected {} minor version", get_span!(protocol)),
    ),
  }
});

state!(response_version_complete, {
  callback!(on_version_complete);
  move_to!(response_status, 0)
});

state!(response_status, {
  // Collect the three digits
  match data {
    [x @ 0x30..=0x39, y @ 0x30..=0x39, z @ 0x30..=0x39, ..] => {
      append!(status, x);
      parser.position += 1;
      append!(status, y);
      parser.position += 1;
      append!(status, z);
      parser.position += 1;
      callback!(on_status, status);
      0
    }
    char!(' ') => move_to!(response_status_complete),
    otherwise!(2) => parser.fail(
      Error::INVALID_STATUS,
      format!("Expected {} response status", get_span!(protocol)),
    ),
    otherwise!(5) => parser.fail(
      Error::INVALID_STATUS,
      format!("Expected {} response status", get_span!(protocol)),
    ),
    _ => suspend!(),
  }
});

state!(response_status_complete, {
  parser.values.status = get_span!(status).parse::<isize>().unwrap();
  callback!(on_status_complete);
  move_to!(response_reason, 0)
});

state!(response_reason, {
  match data {
    // RFC 9112 section 4: HTAB / SP / VCHAR / obs-text
    [x @ (b'\t' | b' ' | 0x21..=0x7e | 0x80..=0xff), ..] => {
      append!(reason, x);
      1
    }
    crlf!() => {
      if !parser.spans.reason.is_empty() {
        callback!(on_reason, reason);
        move_to!(response_reason_complete, 2)
      } else {
        move_to!(header_start, 2)
      }
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Expected status reason"),
    _ => suspend!(),
  }
});

state!(response_reason_complete, {
  callback!(on_reason_complete);
  move_to!(header_start, 0)
});
// #endregion response

// #region headers
fn save_header(parser: &mut Parser, field: &str, raw_value: &str) -> bool {
  let value = raw_value.to_lowercase();

  // Save some headers which impact how we parse the rest of the message
  match field {
    "content-length" => {
      let status = parser.values.status;

      if parser.values.has_chunked_transfer_encoding == 1 {
        fail!(
          UNEXPECTED_CONTENT_LENGTH,
          "Unexpected Content-Length header when Transfer-Encoding header is present"
        );

        return false;
      } else if status / 100 == 1 || status == 204 {
        parser.fail(
          Error::UNEXPECTED_CONTENT_LENGTH,
          format!("Unexpected Content-Length header for a response with status {}", status),
        );

        return false;
      } else if parser.values.expected_content_length != 0 {
        fail!(INVALID_CONTENT_LENGTH, "Invalid duplicate Content-Length header");
        return false;
      } else if let Ok(length) = value.parse::<usize>() {
        parser.values.has_content_length = 1;
        set_value!(expected_content_length, length);
      } else {
        fail!(INVALID_CONTENT_LENGTH, "Invalid Content-Length header");
        return false;
      }
    }
    "transfer-encoding" => {
      let status = parser.values.status;

      if parser.values.expected_content_length > 0 {
        fail!(
          UNEXPECTED_TRANSFER_ENCODING,
          "Unexpected Transfer-Encoding header when Content-Length header is present"
        );

        return false;
      } else if status == 304 {
        // Note that Transfer-Encoding is NOT allowed in 304
        parser.fail(
          Error::UNEXPECTED_TRANSFER_ENCODING,
          format!(
            "Unexpected Transfer-Encoding header for a response with status {}",
            status
          ),
        );

        return false;
      }

      // If chunked is the last encoding
      let value = value.trim();
      if value == "chunked" || value.ends_with(",chunked") || value.ends_with(", chunked") {
        /*
          If this is 1, it means the Transfer-Encoding header was specified more than once.
          This is the second repetition and therefore, the previous one is no longer the last one, making it invalid.
        */
        if parser.values.has_chunked_transfer_encoding == 1 {
          fail!(
            INVALID_TRANSFER_ENCODING,
            "The value \"chunked\" in the Transfer-Encoding header must be the last provided and can be provided only once"
          );

          return false;
        } else {
          parser.values.has_chunked_transfer_encoding = 1;
        }
      } else if parser.values.has_chunked_transfer_encoding == 1 {
        // Any other value when chunked was already specified is invalid
        fail!(
          INVALID_TRANSFER_ENCODING,
          "The value \"chunked\" in the Transfer-Encoding header must be the last provided"
        );

        return false;
      }
    }
    "connection" => match value.as_str() {
      "close" => {
        parser.values.connection = CONNECTION_CLOSE;
      }
      "keep-alive" => {
        parser.values.connection = CONNECTION_KEEPALIVE;
      }
      "upgrade" => {
        parser.values.connection = CONNECTION_UPGRADE;
      }
      _ => (),
    },
    "trailer" => {
      parser.values.has_trailers = 1;
    }
    "upgrade" => {
      parser.values.has_upgrade = 1;
    }
    _ => (),
  }

  true
}

// RFC 9112 section 4
state!(header_start, {
  match data {
    token!(x) => {
      append!(header_field, x);
      1
    }
    [b':', b'\t' | b' ', ..] => {
      callback!(on_header_field, header_field);
      move_to!(header_field_complete_with_space)
    }
    char!(':') => {
      callback!(on_header_field, header_field);
      move_to!(header_field_complete)
    }
    crlf!() => {
      parser.values.continue_without_data = 1;
      move_to!(validate_headers, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid header field name character"),
    _ => suspend!(),
  }
});

state!(header_field_complete, {
  callback!(on_header_field_complete);
  move_to!(header_value_ignore_ows, 0)
});

state!(header_field_complete_with_space, {
  callback!(on_header_field_complete);
  move_to!(header_value_ignore_ows, 1)
});

state!(header_value_ignore_ows, {
  match data {
    [b'\t' | b' ', ..] => 1,
    _ => move_to!(header_value, 0),
  }
});

// RFC 9110 section 5.5 and 5.6
state!(header_value, {
  match data {
    [x @ (b'\t' | b' ' | 0x21..=0x7e | 0x80..=0xff), ..] => {
      append!(header_value, x);
      1
    }
    [b'\r', b'\n', b'\r', b'\n', ..] => {
      if !save_header(
        parser,
        get_span!(header_field).to_lowercase().as_str(),
        get_span!(header_value).as_str(),
      ) {
        return 0;
      }

      callback!(on_header_value, header_value);
      move_to!(header_value_complete_last, 2)
    }
    crlf!() => {
      if !save_header(
        parser,
        get_span!(header_field).to_lowercase().as_str(),
        get_span!(header_value).as_str(),
      ) {
        return 0;
      }

      callback!(on_header_value, header_value);
      clear!(header_field);
      clear!(header_value);
      move_to!(header_value_complete, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid header field value character"),
    _ => suspend!(),
  }
});

state!(header_value_complete, {
  callback!(on_header_value_complete);
  move_to!(header_start, 0)
});

state!(header_value_complete_last, {
  parser.values.continue_without_data = 1;
  callback!(on_header_value_complete);
  move_to!(validate_headers, 2)
});

state!(validate_headers, {
  parser.values.continue_without_data = 1;
  if parser.values.has_upgrade == 1 && parser.values.connection != CONNECTION_UPGRADE {
    parser.values.continue_without_data = 0;

    return parser.fail(
      Error::MISSING_CONNECTION_UPGRADE,
      format!("Missing Connection header set to \"upgrade\" when using the Upgrade header"),
    );
  }

  move_to!(headers_complete, 0)
});

// RFC 9110 section 9.3.6 and 7.8
state!(headers_complete, {
  parser.values.continue_without_data = 1;
  callback!(on_headers_complete);
  move_to!(choose_body, 0)
});

state!(choose_body, {
  parser.values.continue_without_data = 1;

  let method = parser.values.method;
  let status = parser.values.status;

  // In case of Connection: Upgrade
  if parser.values.has_upgrade == 1 {
    if parser.values.connection != CONNECTION_UPGRADE {
      parser.values.continue_without_data = 0;

      return parser.fail(
        Error::MISSING_CONNECTION_UPGRADE,
        format!("Missing Connection header set to \"upgrade\" when using the Upgrade header"),
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
    if parser.values.expected_content_length > 0 {
      parser.values.continue_without_data = 0;

      return parser.fail(
        Error::UNEXPECTED_CONTENT,
        format!("Unexpected content for {} request", method),
      );
    }
  }

  // RFC 9110 section 6.3
  if parser.values.message_type == REQUEST {
    if parser.values.has_content_length == 1 {
      if parser.values.expected_content_length == 0 {
        return complete_message(parser, 0);
      }
    } else if parser.values.has_chunked_transfer_encoding == 0 {
      return complete_message(parser, 0);
    }
  } else {
    if (status < 200 && status != 101) || method == METHOD_HEAD || parser.values.skip_body == 1 {
      return complete_message(parser, 0);
    }

    if parser.values.expected_content_length == 0 {
      if parser.values.has_content_length == 1 {
        return complete_message(parser, 0);
      } else if parser.values.has_chunked_transfer_encoding == 0 {
        return move_to!(body_with_no_length, 0);
      }
    }
  }

  move_to!(body_start, 0)
});

// #endregion headers

// RFC 9110 section 6.4.1
#[inline(always)]
fn complete_message(parser: &mut Parser, advance: isize) -> isize {
  callback!(on_message_complete);

  let connection = parser.values.connection;

  parser.values.clear();
  parser.spans.clear();
  parser.values.continue_without_data = 1;
  parser.values.connection = connection;

  move_to!(message_complete);
  advance
}

// #region common_body
state!(body_start, {
  if parser.values.expected_content_length > 0 {
    parser.values.current_content_length = 0;
    return move_to!(body_via_content_length, 0);
  }

  if parser.values.has_trailers == 1 && !parser.values.has_chunked_transfer_encoding == 0 {
    return fail!(
      UNEXPECTED_TRAILERS,
      "Trailers are not allowed when not using chunked transfer encoding"
    );
  }

  move_to!(chunk_start, 0)
});

// Return PAUSE makes this method idempotent without failing - In this state all data is ignored since we're not in HTTP anymore
state!(tunnel, { suspend!() });

state!(body_complete, { complete_message(parser, 0) });

// #endregion common_body

// #region body via Content-Length
// RFC 9112 section 6.2
state!(body_via_content_length, {
  let remaining = (parser.values.expected_content_length - parser.values.current_content_length) as usize;
  let available = data.len();

  // Less data than what we expect
  if available < remaining {
    parser.spans.body.extend_from_slice(data);
    parser.values.current_content_length += available as isize;

    data_slice_callback!(on_data_body, data, available);
    0
  } else {
    let body = data.get(..remaining).unwrap();
    parser.spans.body.extend_from_slice(body);
    parser.values.current_content_length = parser.values.expected_content_length;

    data_slice_callback!(on_data_body, body, remaining);
    callback!(on_body, body);
    parser.values.continue_without_data = 1;
    move_to!(body_complete, 0)
  }
});
// #endregion body via Content-Length

// RFC 9110 section 6.3
state!(body_with_no_length, {
  let available = data.len();

  parser.spans.body.extend_from_slice(data);
  parser.values.current_content_length += available as isize;

  data_slice_callback!(on_data_body, data, available);
  0
});

// #region body via chunked Transfer-Encoding
// RFC 9112 section 7.1
state!(chunk_start, {
  match data {
    hex_digit!(x) => {
      append!(chunk_length, x);
      1
    }
    char!(';') => {
      if let Ok(length) = isize::from_str_radix(get_span!(chunk_length).as_str(), 16) {
        callback!(on_chunk_length, chunk_length);
        clear!(chunk_length);
        set_value!(expected_chunk_size, length);
        move_to!(chunk_extension_name)
      } else {
        fail!(INVALID_CHUNK_SIZE, "Invalid chunk length")
      }
    }
    crlf!() => {
      if let Ok(length) = isize::from_str_radix(get_span!(chunk_length).as_str(), 16) {
        callback!(on_chunk_length, chunk_length);
        clear!(chunk_length);
        set_value!(expected_chunk_size, length);

        parser.values.continue_without_data = 1;
        move_to!(chunk_check_if_last, 2)
      } else {
        fail!(INVALID_CHUNK_SIZE, "Invalid chunk length")
      }
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid chunk length character"),
    _ => suspend!(),
  }
});

state!(chunk_extension_name, {
  match data {
    token!(x) => {
      append!(chunk_extension_name, x);
      1
    }
    char!('=') => {
      callback!(on_chunk_extension_name, chunk_extension_name);
      move_to!(chunk_extension_value)
    }
    char!(';') => {
      callback!(on_chunk_extension_name, chunk_extension_name);
      clear!(chunk_extension_name);
      move_to!(chunk_extension_name)
    }
    crlf!() => {
      callback!(on_chunk_extension_name, chunk_extension_name);
      clear!(chunk_extension_name);
      clear!(chunk_extension_value);

      parser.values.continue_without_data = 1;
      move_to!(chunk_check_if_last, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension name character"),
    _ => suspend!(),
  }
});

state!(chunk_extension_value, {
  match data {
    token!(x) => {
      append!(chunk_extension_value, x);
      1
    }
    char!('"') => {
      parser.spans.chunk_extension_value.push(b'"');
      move_to!(chunk_extension_quoted_value)
    }
    char!(';') => {
      callback!(on_chunk_extension_value, chunk_extension_value);
      clear!(chunk_extension_name);
      clear!(chunk_extension_value);
      move_to!(chunk_extension_name)
    }
    crlf!() => {
      callback!(on_chunk_extension_value, chunk_extension_value);
      clear!(chunk_extension_name);
      clear!(chunk_extension_value);

      parser.values.continue_without_data = 1;
      move_to!(chunk_check_if_last, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension value character"),
    _ => suspend!(),
  }
});

// RFC 9110 section 5.6.4
state!(chunk_extension_quoted_value, {
  match data {
    [x @ b'\\', y @ b'"', ..] => {
      append!(chunk_extension_value, x);
      parser.position += 1;
      append!(chunk_extension_value, y);
      1
    }
    [x @ b'"', b'\r', b'\n', ..] => {
      append!(chunk_extension_value, x);
      parser.position += 1;
      callback!(on_chunk_extension_value, chunk_extension_value);
      clear!(chunk_extension_name);
      clear!(chunk_extension_value);

      parser.values.continue_without_data = 1;
      move_to!(chunk_check_if_last, 2)
    }
    [x @ b'"', b';', ..] => {
      append!(chunk_extension_value, x);
      parser.position += 1;
      callback!(on_chunk_extension_value, chunk_extension_value);
      clear!(chunk_extension_name);
      clear!(chunk_extension_value);

      parser.values.continue_without_data = 1;
      move_to!(chunk_check_if_last, 1)
    }
    [x @ (b'\t' | b' ' | 0x21 | 0x23..=0x5b | 0x5d..=0x7e), ..] => {
      append!(chunk_extension_value, x);
      1
    }
    otherwise!(3) => fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension quoted value character"),
    _ => suspend!(),
  }
});

state!(chunk_check_if_last, {
  if parser.values.expected_chunk_size == 0 {
    callback!(on_body, body);

    if parser.values.has_trailers == 1 {
      return move_to!(trailer_start, 0);
    } else {
      return move_to!(crlf_after_last_chunk, 0);
    }
  }

  move_to!(chunk_data, 0)
});

state!(chunk_data, {
  let remaining = (parser.values.expected_chunk_size - parser.values.current_chunk_size) as usize;
  let available = data.len();

  // Less data than what we expect
  if available < remaining {
    parser.spans.chunk_data.extend_from_slice(data);
    parser.values.current_chunk_size += available as isize;

    data_slice_callback!(on_data_chunk_data, on_data_body, data, available);
    0
  } else {
    let chunk_data = data.get(..remaining).unwrap();
    parser.spans.chunk_data.extend_from_slice(chunk_data);
    parser.spans.body.extend_from_slice(&parser.spans.chunk_data);

    data_slice_callback!(on_data_chunk_data, on_data_body, chunk_data, remaining);

    callback!(on_chunk_data, chunk_data);
    move_to!(chunk_end, 0)
  }
});

state!(chunk_end, {
  match data {
    crlf!() => {
      parser.values.current_chunk_size = 0;
      clear!(chunk_data);
      move_to!(chunk_start, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Unexpected character after chunk data"),
    _ => suspend!(),
  }
});

state!(crlf_after_last_chunk, {
  match data {
    crlf!() => {
      parser.values.continue_without_data = 1;
      move_to!(body_complete, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Expected CRLF after the last chunk"),
    _ => suspend!(),
  }
});

// #endregion body via chunked Transfer-Encoding

// #region trailers
// RFC 9112 section 7.1.2
state!(trailer_start, {
  match data {
    token!(x) => {
      append!(trailer_field, x);
      1
    }
    [b':', b'\t' | b' ', ..] => {
      callback!(on_trailer_field, trailer_field);
      move_to!(trailer_value_ignore_ows, 2)
    }
    char!(':') => {
      callback!(on_trailer_field, trailer_field);
      move_to!(trailer_value_ignore_ows)
    }
    crlf!() => {
      callback!(on_trailers_complete);
      parser.values.continue_without_data = 1;
      move_to!(trailers_complete, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid trailer field name character"),
    _ => suspend!(),
  }
});

state!(trailer_value_ignore_ows, {
  match data {
    [b'\t' | b' ', ..] => 1,
    _ => move_to!(trailer_value, 0),
  }
});

state!(trailer_value, {
  match data {
    [x @ (b'\t' | b' ' | 0x21..=0x7e | 0x80..=0xff), ..] => {
      append!(trailer_value, x);
      1
    }
    [b'\r', b'\n', b'\r', b'\n', ..] => {
      callback!(on_trailer_value, trailer_value);
      parser.values.continue_without_data = 1;
      move_to!(trailers_complete, 4)
    }
    crlf!() => {
      callback!(on_trailer_value, trailer_value);
      clear!(trailer_field);
      clear!(trailer_value);
      move_to!(trailer_start, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid trailer field value character"),
    _ => suspend!(),
  }
});

state!(trailers_complete, {
  callback!(on_trailers_complete);
  complete_message(parser, 0)
});
// #endregion trailers

state!(message_complete, {
  let must_close = parser.values.connection == CONNECTION_CLOSE;
  parser.values.connection = 0;

  if must_close {
    move_to!(finish, 0)
  } else {
    move_to!(start, 0)
  }
});

generate_parser!({
  pub fn get_span(&self, span: &Vec<u8>) -> String {
    unsafe { String::from_utf8_unchecked((*span).clone()) }
  }

  fn move_to(&mut self, state: State, advance: isize) -> isize {
    #[cfg(debug_assertions)]
    {
      let fail_advance = if advance < 0 { advance } else { -advance };

      // Notify the end of the current state
      let result = if let Some(cb) = self.callbacks.before_state_change {
        cb(self, std::ptr::null(), 0)
      } else {
        0
      };

      match result {
        0 => (),
        -1 => return fail_advance,
        _ => {
          return self.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
        }
      };
    };

    // Change the state
    self.state = state;

    #[cfg(debug_assertions)]
    {
      let fail_advance = if advance < 0 { advance } else { -advance };

      let result = if let Some(cb) = self.callbacks.after_state_change {
        cb(self, std::ptr::null(), 0)
      } else {
        0
      };

      match result {
        0 => advance,
        -1 => fail_advance,
        _ => {
          return self.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
        }
      }
    };

    advance
  }

  fn fail(&mut self, code: Error, reason: String) -> isize {
    self.error_code = code;
    self.error_description = reason;
    self.state = State::ERROR;

    0
  }

  fn fail_str(&mut self, code: Error, reason: &str) -> isize {
    self.fail(code, reason.into())
  }

  pub fn pause(&mut self) {
    self.paused = true;
  }

  pub fn resume(&mut self) {
    self.paused = false;
  }

  pub fn finish(&mut self) {
    match self.state {
      State::START | State::REQUEST_START | State::RESPONSE_START | State::FINISH => {
        self.state = State::FINISH;
      }
      State::BODY_WITH_NO_LENGTH => {
        if let Some(cb) = self.callbacks.on_message_complete {
          let action = cb(self, std::ptr::null(), 0);

          if action != 0 {
            self.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
          }
        }

        self.state = State::FINISH;
      }
      State::ERROR => (),
      _ => {
        self.fail_str(Error::UNEXPECTED_EOF, "Unexpected end of data");
      }
    };
  }
});

generate_parser_interface!({
  #[no_mangle]
  pub extern "C" fn free_string(s: *mut c_char) {
    unsafe {
      if s.is_null() {
        return;
      }

      let _ = CString::from_raw(s);
    }
  }

  #[no_mangle]
  pub extern "C" fn create_parser() -> *mut Parser {
    Box::into_raw(Box::new(Parser::new()))
  }

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
  pub extern "C" fn execute_parser(parser: *mut Parser, data: *const c_char, limit: usize) -> usize {
    unsafe { parser.as_mut().unwrap().parse(data, limit) }
  }

  #[no_mangle]
  pub extern "C" fn pause_parser(parser: *mut Parser) {
    unsafe { parser.as_mut().unwrap().pause() }
  }

  #[no_mangle]
  pub extern "C" fn resume_parser(parser: *mut Parser) {
    unsafe { parser.as_mut().unwrap().resume() }
  }

  #[no_mangle]
  pub extern "C" fn finish_parser(parser: *mut Parser) {
    unsafe { parser.as_mut().unwrap().finish() }
  }

  #[no_mangle]
  pub extern "C" fn is_paused(parser: *mut Parser) -> bool {
    unsafe { parser.as_mut().unwrap().paused }
  }

  #[no_mangle]
  pub extern "C" fn get_owner(parser: *mut Parser) -> *mut c_void {
    unsafe {
      match parser.as_mut().unwrap().owner {
        Some(x) => x,
        None => std::ptr::null_mut(),
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
  pub extern "C" fn get_state(parser: *mut Parser) -> u8 {
    unsafe { parser.as_mut().unwrap().state as u8 }
  }

  #[no_mangle]
  pub extern "C" fn get_position(parser: *mut Parser) -> usize {
    unsafe { (*parser).position }
  }

  #[no_mangle]
  pub extern "C" fn get_error_code(parser: *mut Parser) -> u8 {
    unsafe { (*parser).error_code as u8 }
  }

  #[no_mangle]
  pub extern "C" fn get_error_code_description(parser: *mut Parser) -> *mut c_char {
    unsafe {
      std::ffi::CString::new((*parser).error_description.clone())
        .unwrap()
        .into_raw()
    }
  }
});
