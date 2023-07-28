use milo_parser_generator::{
  append, callback, callbacks, char, clear, crlf, digit, errors, fail, generate_parser, get_span, hex_digit, measure,
  method, move_to, otherwise, pause, set_value, spans, state, string, token, url, values,
};

pub mod test_utils;

pub const AUTODETECT: isize = 0;
pub const REQUEST: isize = 1;
pub const RESPONSE: isize = 2;

pub const CONNECTION_KEEPALIVE: isize = 0;
pub const CONNECTION_CLOSE: isize = 1;
pub const CONNECTION_UPGRADE: isize = 2;

values!(
  message_type,
  is_connect_request,
  connection,
  expected_content_length,
  expected_chunk_size,
  has_chunked_transfer_encoding,
  has_upgrade,
  has_trailers,
  current_content_length,
  current_chunk_size
);

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
  // TODO@PI: Check other _complete callbacks
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

// #region request_or_response
// Depending on the mode flag, choose the initial state
state!(start, {
  match parser.values.mode {
    AUTODETECT => move_to!(message_start @ 0),
    REQUEST => move_to!(request_start @ 0),
    RESPONSE => move_to!(response_start @  0),
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
      move_to!(request_start @ 0)
    }
    string!("HTTP/") | string!("RTSP/") => {
      parser.values.message_type = RESPONSE;
      callback!(on_message_start);
      callback!(on_response);
      move_to!(response_start @ 0)
    }
    otherwise!(5) => fail!(UNEXPECTED_CHARACTER, "Invalid word"),
    _ => pause!(),
  }
});
// #endregion

// #region request
// RFC 9112 section 3
state!(request_start, {
  match data {
    crlf!() => 2, // RFC 9112 section 2.2 - Repeated
    token!(x) => {
      append!(method, x @ request_method)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Expected method"),
    _ => pause!(),
  }
});

// RFC 9112 section 3.1
state!(request_method, {
  match data {
    token!(x) => append!(method, x),
    char!(' ') => {
      if get_span!(method) == "CONNECT" {
        parser.values.is_connect_request = 1;
      }

      callback!(on_method, method @ request_method_complete)
    }
    _ => fail!(UNEXPECTED_CHARACTER, "Expected token character"),
  }
});

state!(request_method_complete, {
  callback!(on_method_complete @ request_url, 0)
});

// RFC 9112 section 3.2
state!(request_url, {
  match data {
    url!(x) => append!(url, x),
    char!(' ') => callback!(on_url, url @ request_url_complete),
    _ => fail!(UNEXPECTED_CHARACTER, "Expected URL character"),
  }
});

state!(request_url_complete, {
  callback!(on_url_complete @ request_protocol, 0)
});

// RFC 9112 section 2.3
state!(request_protocol, {
  match data {
    string!("HTTP/") => {
      parser.spans.protocol = b"HTTP".to_vec();
      callback!(on_protocol, protocol @ request_protocol_complete, 5)
    }
    string!("RTSP/") => {
      parser.spans.protocol = b"RTSP".to_vec();
      callback!(on_protocol, protocol @ request_protocol_complete, 5)
    }
    otherwise!(5) => fail!(UNEXPECTED_CHARACTER, "Expected protocol"),
    _ => pause!(),
  }
});

state!(request_protocol_complete, {
  callback!(on_protocol_complete @ request_version_major, 0)
});

state!(request_version_major, {
  match data {
    digit!(x) => append!(version, x),
    [x @ b'.', ..] => {
      append!(version, x);
      callback!(on_data_version, version @ request_version_minor)
    }
    _ => parser.fail(
      Error::UNEXPECTED_CHARACTER,
      format!("Expected {} minor version", get_span!(protocol)),
    ),
  }
});

state!(request_version_minor, {
  match data {
    digit!(x) => append!(version, x),
    crlf!() => {
      // Validate the version
      match parser.spans.version[..] {
        string!("1.1") | string!("2.0") => callback!(on_version, version @ request_version_complete, 2),
        _ => fail!(INVALID_VERSION, "Invalid HTTP version"),
      }
    }
    otherwise!(2) => parser.fail(
      Error::UNEXPECTED_CHARACTER,
      format!("Expected {} minor version", get_span!(protocol)),
    ),
    _ => pause!(),
  }
});

state!(request_version_complete, {
  callback!(on_version_complete @ header_start, 0)
});
// #endregion request

// #region response
// RFC 9112 section 4
state!(response_start, {
  match data {
    crlf!() => 2, // RFC 9112 section 2.2 - Repeated
    string!("HTTP/") => {
      parser.spans.protocol = b"HTTP".to_vec();
      callback!(on_protocol, protocol @ response_protocol_complete, 5)
    }
    string!("RTSP/") => {
      parser.spans.protocol = b"RTSP".to_vec();
      callback!(on_protocol, protocol @ response_protocol_complete, 5)
    }
    otherwise!(5) => {
      fail!(UNEXPECTED_CHARACTER, "Expected protocol")
    }
    _ => pause!(),
  }
});

state!(response_protocol_complete, {
  callback!(on_protocol_complete @ response_version_major, 0)
});

state!(response_version_major, {
  match data {
    digit!(x) => append!(version, x),
    [x @ b'.', ..] => {
      append!(version, x);
      callback!(on_data_version, version @ response_version_minor)
    }
    _ => parser.fail(
      Error::UNEXPECTED_CHARACTER,
      format!("Expected {} minor version", unsafe {
        String::from_utf8_unchecked(parser.spans.protocol.clone())
      }),
    ),
  }
});

state!(response_version_minor, {
  match data {
    digit!(x) => append!(version, x),
    char!(' ') => {
      // Validate the version
      match parser.spans.version[..] {
        string!("1.1") | string!("2.0") => callback!(on_version, version @ response_version_complete),
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
  callback!(on_version_complete @ response_status, 0)
});

state!(response_status, {
  // Collect the three digits
  match data {
    [x @ 0x30..=0x39, y @ 0x30..=0x39, z @ 0x30..=0x39, ..] => {
      append!(status, x);
      append!(status, y);
      append!(status, z);
      callback!(on_status, status @ 3)
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
    _ => pause!(),
  }
});

state!(response_status_complete, {
  callback!(on_status_complete @ response_reason, 0)
});

state!(response_reason, {
  match data {
    // RFC 9112 section 4: HTAB / SP / VCHAR / obs-text
    [x @ (b'\t' | b' ' | 0x21..=0x7e | 0x80..=0xff), ..] => append!(reason, x),
    crlf!() if !parser.spans.reason.is_empty() => {
      callback!(on_reason, reason @ response_reason_complete, 2)
    }
    otherwise!(5) => fail!(UNEXPECTED_CHARACTER, "Expected status reason"),
    _ => pause!(),
  }
});

state!(response_reason_complete, {
  callback!(on_reason_complete @ header_start, 0)
});
// #endregion response

// #region headers
fn save_header(parser: &mut Parser, field: &str, value: &str) {
  // Save some headers which impact how we parse the rest of the message
  match field {
    "content-length" => {
      let status = get_span!(status);

      if parser.values.has_chunked_transfer_encoding == 1 {
        fail!(
          UNEXPECTED_CONTENT_LENGTH,
          "Unexpected Content-Length header when Transfer-Encoding header is present"
        );
        return;
      } else if status.starts_with("1") || status.starts_with("204") || status.starts_with("304") {
        parser.fail(
          Error::UNEXPECTED_CONTENT_LENGTH,
          format!("Unexpected Content-Length header for a response with status {}", status),
        );
        return;
      }

      if let Ok(length) = value.parse::<usize>() {
        set_value!(expected_content_length, length);
      } else {
        fail!(INVALID_CONTENT_LENGTH, "Invalid Content-Length header");
      }
    }
    "transfer-encoding" => {
      let status = get_span!(status);

      if parser.values.expected_content_length > 0 {
        fail!(
          UNEXPECTED_TRANSFER_ENCODING,
          "Unexpected Transfer-Encoding header when Content-Length header is present"
        );
        return;
      } else if status.starts_with("1") || status.starts_with("204") {
        // Note that Transfer-Encoding is allowed in 304
        parser.fail(
          Error::UNEXPECTED_TRANSFER_ENCODING,
          format!(
            "Unexpected Transfer-Encoding header for a response with status {}",
            status
          ),
        );
        return;
      }

      parser.values.has_chunked_transfer_encoding = 1;

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
}

// RFC 9112 section 4
state!(header_start, {
  match data {
    token!(x) => append!(header_field, x),
    [b':', b'\t' | b' ', ..] => callback!(on_header_field, header_field @ header_field_complete_with_space, 1),
    char!(':') => callback!(on_header_field, header_field @ header_field_complete),
    crlf!() => move_to!(headers_complete @ 2),
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid header field name character"),
    _ => pause!(),
  }
});

state!(header_field_complete, {
  callback!(on_header_field_complete @ header_value, 0)
});
state!(header_field_complete_with_space, {
  callback!(on_header_field_complete @ header_value, 1)
});

// RFC 9110 section 5.5 and 5.6
state!(header_value, {
  match data {
    [x @ (b'\t' | b' ' | 0x21..=0x7e | 0x80..=0xff), ..] => append!(header_value, x),
    [b'\r', b'\n', b'\r', b'\n', ..] => {
      save_header(
        parser,
        get_span!(header_field).to_lowercase().as_str(),
        get_span!(header_value).as_str(),
      );

      callback!(on_header_value, header_value @ header_value_complete_last, 2)
    }
    crlf!() => {
      save_header(
        parser,
        get_span!(header_field).to_lowercase().as_str(),
        get_span!(header_value).as_str(),
      );
      callback!(on_header_value, header_value);
      clear!(header_field @ 0);
      clear!(header_value @ 0);
      move_to!(header_value_complete @ 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid header field value character"),
    _ => pause!(),
  }
});

state!(header_value_complete, {
  callback!(on_header_value_complete @ header_start, 0)
});
state!(header_value_complete_last, {
  parser.values.parse_empty_data = 1;
  callback!(on_header_value_complete @ headers_complete, 2)
});
state!(headers_complete, {
  parser.values.parse_empty_data = 1;
  callback!(on_headers_complete @ body_start, 0)
});

// #endregion headers

// RFC 9110 section 6.4.1
#[inline(always)]
fn restart(parser: &mut Parser, advance: isize) -> isize {
  parser.values.clear();
  parser.spans.clear();

  move_to!(start);

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

    return callback!(on_upgrade @ body_upgrade, 0);
  }

  if parser.values.is_connect_request == 1 {
    return callback!(on_upgrade @ body_upgrade, 0);
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
    callback!(on_message_complete);
    return restart(parser, 0);
  }

  if parser.values.expected_content_length > 0 {
    parser.values.current_content_length = 0;
    return move_to!(body_via_content_length @ 0);
  }

  if parser.values.has_trailers == 1 && !parser.values.has_chunked_transfer_encoding == 0 {
    return fail!(
      UNEXPECTED_TRAILERS,
      "Trailers are not allowed when not using chunked transfer encoding"
    );
  }

  move_to!(chunk_start @ 0)
});

// Return MIN makes this method idempotent without failing
state!(body_upgrade, { pause!() });

state!(body_complete, {
  callback!(on_message_complete);
  restart(parser, 0)
});

// #endregion common_body

// #region body via Content-Length
// RFC 9112 section 6.2
state!(body_via_content_length, {
  let remaining = (parser.values.expected_content_length - parser.values.current_content_length) as usize;
  let available = data.len();

  // Less data than what we expect
  if available < remaining {
    println!("AVAILABLE");
    parser.spans.body.extend_from_slice(data);
    parser.values.current_content_length += available as isize;

    available as isize
  } else {
    let missing = data.get(..remaining).unwrap();
    parser.spans.body.extend_from_slice(missing);

    callback!(on_body, body @ body_complete);
    remaining as isize
  }
});

// #endregion body via Content-Length

// #region body via chunked Transfer-Encoding
// RFC 9112 section 7.1
state!(chunk_start, {
  match data {
    hex_digit!(x) => append!(chunk_length, x),
    char!(';') => {
      if let Ok(length) = isize::from_str_radix(get_span!(chunk_length).as_str(), 16) {
        callback!(on_chunk_length, chunk_length);
        clear!(chunk_length);
        set_value!(expected_chunk_size, length @ chunk_extension_name)
      } else {
        fail!(INVALID_CHUNK_SIZE, "Invalid chunk length")
      }
    }
    crlf!() => {
      if let Ok(length) = isize::from_str_radix(get_span!(chunk_length).as_str(), 16) {
        callback!(on_chunk_length, chunk_length);
        clear!(chunk_length);
        set_value!(expected_chunk_size, length @ chunk_data, 2)
      } else {
        fail!(INVALID_CHUNK_SIZE, "Invalid chunk length")
      }
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid chunk length character"),
    _ => pause!(),
  }
});

state!(chunk_extension_name, {
  match data {
    token!(x) => append!(chunk_extension_name, x),
    char!('=') => callback!(on_chunk_extension_name, chunk_extension_name @ chunk_extension_value),
    char!(';') => {
      callback!(on_chunk_extension_name, chunk_extension_name);
      clear!(chunk_extension_name @ chunk_extension_name)
    }
    crlf!() => {
      callback!(on_chunk_extension_name, chunk_extension_name);
      clear!(chunk_extension_name);
      clear!(chunk_extension_value);
      move_to!(chunk_data @ 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension name character"),
    _ => pause!(),
  }
});

state!(chunk_extension_value, {
  match data {
    token!(x) => append!(chunk_extension_value, x),
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
      move_to!(chunk_data @ 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension value character"),
    _ => pause!(),
  }
});

// RFC 9110 section 5.6.4
state!(chunk_extension_quoted_value, {
  match data {
    [x @ b'\\', y @ b'"', ..] => {
      append!(chunk_extension_value, x);
      append!(chunk_extension_value, y);
      2
    }
    [x @ b'"', b'\r', b'\n', ..] => {
      append!(chunk_extension_value, x);
      callback!(on_chunk_extension_value, chunk_extension_value);
      clear!(chunk_extension_name);
      clear!(chunk_extension_value);
      move_to!(chunk_data @ 3)
    }
    [x @ b'"', b';', ..] => {
      append!(chunk_extension_value, x);
      callback!(on_chunk_extension_value, chunk_extension_value);
      clear!(chunk_extension_name);
      clear!(chunk_extension_value);
      move_to!(chunk_data @ 2)
    }
    [x @ (b'\t' | b' ' | 0x21 | 0x23..=0x5b | 0x5d..=0x7e), ..] => append!(chunk_extension_value, x),
    otherwise!(3) => fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension quoted value character"),
    _ => pause!(),
  }
});

state!(chunk_data, {
  if parser.values.expected_chunk_size == 0 {
    if parser.values.has_trailers == 1 {
      return callback!(on_body, body @ trailer_start, 0);
    } else {
      return callback!(on_body, body @ body_complete, 0);
    }
  }

  let remaining = (parser.values.expected_chunk_size - parser.values.current_chunk_size) as usize;
  let available = data.len();

  // Less data than what we expect
  if available < remaining {
    println!("AVAILABLE CHUNK");
    parser.spans.chunk_data.extend_from_slice(data);
    parser.values.current_chunk_size += available as isize;

    available as isize
  } else {
    let missing = data.get(..remaining).unwrap();
    parser.spans.chunk_data.extend_from_slice(missing);
    parser.spans.body.extend_from_slice(&parser.spans.chunk_data);

    callback!(on_chunk_data, chunk_data @ chunk_end);
    remaining as isize
  }
});

state!(chunk_end, {
  match data {
    crlf!() => {
      parser.values.current_chunk_size = 0;
      clear!(chunk_data @ chunk_start, 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Unexpected character after chunk data"),
    _ => pause!(),
  }
});

// #endregion body via chunked Transfer-Encoding

// #region trailers
// RFC 9112 section 7.1.2
state!(trailer_start, {
  match data {
    token!(x) => append!(trailer_field, x),
    [b':', b'\t' | b' ', ..] => callback!(on_trailer_field, trailer_field @ trailer_value, 2),
    char!(':') => callback!(on_trailer_field, trailer_field @ trailer_value),
    crlf!() => callback!(on_trailers_complete @ body_complete, 2),
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid trailer field name character"),
    _ => pause!(),
  }
});

state!(trailer_value, {
  match data {
    [x @ (b'\t' | b' ' | 0x21..=0x7e | 0x80..=0xff), ..] => append!(trailer_value, x),
    [b'\r', b'\n', b'\r', b'\n', ..] => {
      callback!(on_trailer_value, trailer_value);
      callback!(on_trailers_complete @ body_complete, 4)
    }
    crlf!() => {
      callback!(on_trailer_value, trailer_value);
      clear!(trailer_field @ 0);
      clear!(trailer_value @ 0);
      move_to!(trailer_start @ 2)
    }
    otherwise!(2) => fail!(UNEXPECTED_CHARACTER, "Invalid trailer field value character"),
    _ => pause!(),
  }
});
// #endregion trailers

generate_parser!();
