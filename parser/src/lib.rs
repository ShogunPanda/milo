#[macro_use]
extern crate lazy_static;

use milo_parser_generator::{
  append, callback, callbacks, char, clear, crlf, data_slice_callback, digit, errors, fail, generate_parser, get_span,
  hex_digit, method, move_to, otherwise, persistent_values, set_value, settable_values, spans, state, string, suspend,
  token, url, values,
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
  message_type,
  is_connect_request,
  method,
  status,
  version_major,
  version_minor,
  connection,
  expected_content_length,
  expected_chunk_size,
  has_chunked_transfer_encoding,
  has_upgrade,
  has_trailers,
  current_content_length,
  current_chunk_size
);

persistent_values!(id, mode);

settable_values!(id, mode);

spans!(
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
  MISSING_CONNECTION_UPGRADE
);

callbacks!(
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

// #region request_or_response
// Depending on the mode flag, choose the initial state
state!(start, {
  match parser.values.mode {
    AUTODETECT => move_to!(message_start, 0),
    REQUEST => {
      callback!(on_message_start);
      move_to!(request_start, 0)
    }
    RESPONSE => {
      callback!(on_message_start);
      move_to!(response_start, 0)
    }
    _ => fail!(UNEXPECTED_CHARACTER, "Invalid mode"),
  }
});

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
// #endregion

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
      let method_str = get_span!(method);

      if get_span!(method) == "CONNECT" {
        parser.values.is_connect_request = 1;
      }

      parser.values.method = method_as_int(std::ffi::CString::new(method_str).unwrap().into_raw());

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
    crlf!() if !parser.spans.reason.is_empty() => {
      callback!(on_reason, reason);
      move_to!(response_reason_complete, 2)
    }
    otherwise!(5) => fail!(UNEXPECTED_CHARACTER, "Expected status reason"),
    _ => suspend!(),
  }
});

state!(response_reason_complete, {
  callback!(on_reason_complete);
  move_to!(header_start, 0)
});
// #endregion response

// #region headers
fn save_header(parser: &mut Parser, field: &str, value: &str) -> bool {
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
      } else if status % 100 == 1 || status == 204 || status == 304 {
        parser.fail(
          Error::UNEXPECTED_CONTENT_LENGTH,
          format!("Unexpected Content-Length header for a response with status {}", status),
        );

        return false;
      }

      if let Ok(length) = value.parse::<usize>() {
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
      } else if status % 100 == 1 || status == 304 {
        // Note that Transfer-Encoding is allowed in 304
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
      if value.ends_with("chunked") || value.ends_with(",chunked") || value.ends_with(", chunked") {
        /*
          If this is 1, it means the Transfer-Encoding header was specified more than once.
          This is the second repetition and therefore, the previous one is no longer the last one, making it invalid.
        */
        if parser.values.has_chunked_transfer_encoding == 1 {
          fail!(
            INVALID_TRANSFER_ENCODING,
            "The value \"chunked\" in the Transfer-Encoding header must be the last provided"
          );

          return false;
        } else {
          parser.values.has_chunked_transfer_encoding = 1;
        }
      }

      // Check that chunked is the last provided encoding
      if value != "chunked"
        && (value.starts_with("chunked,") || value.contains(",chunked,") || value.contains(", chunked,"))
      {
        fail!(
          INVALID_TRANSFER_ENCODING,
          "The value \"chunked\" in the Transfer-Encoding header must be the last provided"
        );

        return false;
      }
    }
    "connection" => match value {
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
      move_to!(headers_complete, 2)
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
  move_to!(headers_complete, 2)
});

state!(headers_complete, {
  parser.values.continue_without_data = 1;
  callback!(on_headers_complete);
  move_to!(body_start, 0)
});

// #endregion headers

// RFC 9110 section 6.4.1
#[inline(always)]
fn complete_message(parser: &mut Parser, advance: isize) -> isize {
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
  let method = get_span!(method);

  // In case of Connection: Upgrade
  if parser.values.has_upgrade == 1 {
    if parser.values.connection != CONNECTION_UPGRADE {
      return parser.fail(
        Error::MISSING_CONNECTION_UPGRADE,
        format!("Missing Connection header set to \"upgrade\" when using the Upgrade header"),
      );
    }

    callback!(on_upgrade);
    parser.values.continue_without_data = 1;
    return move_to!(start_tunnel, 0);
  }

  if parser.values.is_connect_request == 1 {
    parser.values.continue_without_data = 1;
    return move_to!(start_tunnel, 0);
  }

  if method == "GET" || method == "HEAD" {
    if parser.values.expected_content_length > 0 {
      return parser.fail(
        Error::UNEXPECTED_CONTENT,
        format!("Unexpected content for {} request", method),
      );
    }
  }

  if parser.values.expected_content_length == 0 && parser.values.has_chunked_transfer_encoding == 0 {
    return complete_message(parser, 0);
  }

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

// RFC 9110 section 9.3.6 and 7.8
state!(start_tunnel, {
  callback!(on_message_complete);
  move_to!(tunnel, 0)
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

    data_slice_callback!(on_data_chunk_data, data);
    0
  } else {
    let body = data.get(..remaining).unwrap();
    parser.spans.body.extend_from_slice(body);

    data_slice_callback!(on_data_body, body);
    callback!(on_body, body);
    parser.values.continue_without_data = 1;
    move_to!(body_complete, 0)
  }
});

// #endregion body via Content-Length

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
      return move_to!(body_complete, 0);
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

    data_slice_callback!(on_data_chunk_data, on_data_body, data);
    0
  } else {
    let chunk_data = data.get(..remaining).unwrap();
    parser.spans.chunk_data.extend_from_slice(chunk_data);
    parser.spans.body.extend_from_slice(&parser.spans.chunk_data);

    data_slice_callback!(on_data_chunk_data, on_data_body, chunk_data);

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

  callback!(on_message_complete);

  if must_close {
    move_to!(finish, 0)
  } else {
    move_to!(start, 0)
  }
});

generate_parser!();
