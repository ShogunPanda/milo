#![allow(clippy::not_unsafe_ptr_arg_deref)]

extern crate alloc;

use alloc::vec::Vec;
use alloc::{boxed::Box, format};
use core::cell::{Cell, RefCell};
use core::ffi::{c_char, c_uchar, c_void};
use core::fmt::Debug;
use core::ptr;
use core::str;
use core::{slice, slice::from_raw_parts};
#[cfg(any(debug_assertions, feature = "debug"))]
use std::time::Instant;

use milo_macros::*;

use crate::*;

/// Returns the index of the first character in the buffer that does not match
/// the provided table. If all characters match, it returns None.
#[inline]
fn consume_new(buf: &[u8], table: &[bool; 256]) -> Option<usize> {
  let len = buf.len();
  let mut i = 0;

  while i + 16 <= len {
    let p = &buf[i..i + 16];

    let ok = table[p[0] as usize]
      & table[p[1] as usize]
      & table[p[2] as usize]
      & table[p[3] as usize]
      & table[p[4] as usize]
      & table[p[5] as usize]
      & table[p[6] as usize]
      & table[p[7] as usize]
      & table[p[8] as usize]
      & table[p[9] as usize]
      & table[p[10] as usize]
      & table[p[11] as usize]
      & table[p[12] as usize]
      & table[p[13] as usize]
      & table[p[14] as usize]
      & table[p[15] as usize];

    if !ok {
      for j in 0..16 {
        if !table[p[j] as usize] {
          return Some(i + j);
        }
      }
    }

    i += 16;
  }

  while i < len {
    if !table[buf[i] as usize] {
      return Some(i);
    }

    i += 1;
  }

  None
}

impl Parser {
  /// Parses a slice of characters.
  ///
  /// It returns the number of consumed characters.
  pub fn parse(&mut self, input: *const c_uchar, limit: usize) -> usize {
    // If the self.is paused, this is a no-op
    if self.paused {
      return 0;
    }

    let input = unsafe { from_raw_parts(input, limit) };

    // Set the data to analyze, prepending unconsumed data from previous iteration
    // if needed
    let mut limit = limit;
    let aggregate: Vec<c_uchar>;
    let unconsumed_len = self.unconsumed_len;

    let mut data = if self.manage_unconsumed && unconsumed_len > 0 {
      unsafe {
        limit += unconsumed_len;
        let unconsumed = from_raw_parts(self.unconsumed, unconsumed_len);

        aggregate = [unconsumed, input].concat();
        &aggregate[..]
      }
    } else {
      input
    };

    // Limit the data that is currently analyzed
    data = &data[..limit];
    let mut available = data.len();

    #[cfg(any(debug_assertions, feature = "debug"))]
    let mut last = Instant::now();

    #[cfg(any(debug_assertions, feature = "debug"))]
    let start = Instant::now();

    #[cfg(any(debug_assertions, feature = "debug"))]
    let mut previous_state = self.state;

    #[cfg(any(debug_assertions, feature = "debug"))]
    let previous_position = self.position;

    // States will advance position manually, the parser has to explicitly
    // track it
    self.position = 0;
    let mut advanced: usize;
    let mut parsing = true;

    #[cfg(any(debug_assertions, feature = "debug"))]
    eprintln!("[milo::debug] loop enter");

    // Until there is data or there is a request to continue
    'parser: while parsing
      && (!self.paused || self.state == STATE_COMPLETE)
      && (available != 0 || self.continue_without_data)
    {
      #[cfg(any(debug_assertions, feature = "debug"))]
      {
        eprintln!(
          "[milo::debug] loop before processing: previous_position={}, position={}, available={}, \
           continue_without_data={}",
          previous_position, self.position, available, self.continue_without_data
        );
      }

      // Reset the continue_without_data flag
      self.continue_without_data = false;
      advanced = 0;

      'state: {
        match self.state {
          // Depending on the mode flag, choose the initial state
          state!(start) => {
            match self.mode {
              MESSAGE_TYPE_AUTODETECT => {
                move_to!(autodetect);
              }
              MESSAGE_TYPE_REQUEST => {
                self.message_type = MESSAGE_TYPE_REQUEST;
                callback!(on_message_start);
                move_to!(request);
              }
              MESSAGE_TYPE_RESPONSE => {
                self.message_type = MESSAGE_TYPE_RESPONSE;
                callback!(on_message_start);
                move_to!(response);
              }
              _ => {
                fail!(UNEXPECTED_CHARACTER, "Invalid mode");
              }
            }
          }

          // If the parser has finished and it receives more data, error
          state!(finish) => {
            fail!(UNEXPECTED_CHARACTER, "Unexpected data");
          }

          // The error state is a no-op
          state!(error) => {
            suspend!();
          }

          // Autodetect if there is a HTTP/RTSP method or a response
          state!(autodetect) => {
            if data.len() >= 5 && data[4] == b'/' && (data.starts_with(b"HTTP") || data.starts_with(b"RTSP")) {
              self.message_type = MESSAGE_TYPE_RESPONSE;
              callback!(on_response);
              callback!(on_message_start);
              move_to!(response);
            } else if crlf_new!() {
              // RFC 9112 section 2.2
              advance!(2);
            } else {
              // For performance reason, we assume it's a request so we don't lookup the
              // method twice
              self.message_type = MESSAGE_TYPE_REQUEST;
              callback!(on_request);
              callback!(on_message_start);
              move_to!(request);
            }
          }
          // #endregion general

          // #region request - Request line parsing
          // RFC 9112 section 3
          state!(request) => {
            if data.len() == 0 {
              suspend!();
            } else if crlf_new!() {
              // RFC 9112 section 2.2 - Repeated
              advance!(2);
            } else if token_new!() {
              self.clear();
              move_to!(request_method);
            } else {
              fail!(UNEXPECTED_CHARACTER, "Expected method or CRLF");
            }
          }

          // RFC 9112 section 3.1
          state!(request_method) => {
            match consume_new(data, &TOKEN_TABLE) {
              Some(0) => {
                fail!(UNEXPECTED_CHARACTER, "Expected method character");
              }
              Some(valid) => {
                if data[valid] != b' ' {
                  fail!(UNEXPECTED_CHARACTER, "Expected space after request method");
                } else {
                  if let Some(&method) = METHODS.get(&data[..valid]) {
                    self.method = method;

                    if method == METHOD_CONNECT {
                      self.is_connect = true;
                    }
                  } else {
                    fail!(UNEXPECTED_CHARACTER, "Invalid method");
                  }

                  callback!(on_method, valid);
                  advance!(valid + 1);
                  move_to!(request_url);
                }
              }
              None => {
                // Entire buffer consumed, wait for more data
                break 'parser;
              }
            }
          }

          // RFC 9112 section 3.2
          state!(request_url) => {
            match consume_new(data, &URL_TABLE) {
              Some(0) => {
                fail!(UNEXPECTED_CHARACTER, "Expected URL character");
              }
              Some(valid) => {
                if data[valid] != b' ' {
                  fail!(UNEXPECTED_CHARACTER, "Expected space after request URL");
                } else {
                  callback!(on_url, valid);
                  advance!(valid + 1);
                  move_to!(request_protocol);
                }
              }
              None => {
                // Entire buffer consumed, wait for more data
                break 'parser;
              }
            }
          }

          // RFC 9112 section 2.3
          state!(request_protocol) => {
            if data.len() >= 5 {
              if data[4] == b'/' && (data.starts_with(b"HTTP") || data.starts_with(b"RTSP")) {
                callback!(on_protocol, 4);
                advance!(5);
                move_to!(request_version);
              } else {
                fail!(UNEXPECTED_CHARACTER, "Expected protocol");
              }
            } else {
              suspend!();
            }
          }

          state!(request_version) => {
            if data.len() >= 5 {
              if DIGIT_TABLE[data[0] as usize]
                && data[1] == b'.'
                && DIGIT_TABLE[data[2] as usize]
                && data[3] == b'\r'
                && data[4] == b'\n'
              {
                if data[0] == b'1' && data[2] == b'1' {
                  self.version_major = 1;
                  self.version_minor = 1;
                } else if data[0] == b'2' && data[2] == b'0' {
                  self.version_major = 2;
                  self.version_minor = 0;

                  if self.method == METHOD_PRI {
                    fail!(UNSUPPORTED_HTTP_VERSION, "HTTP/2.0 is not supported");
                  }
                } else {
                  fail!(INVALID_VERSION, "Invalid HTTP version");
                }

                callback!(on_version, 3);
                advance!(5);
                move_to!(header_name);
              } else {
                fail!(UNEXPECTED_CHARACTER, "Expected HTTP version");
              }
            } else {
              suspend!();
            }
          }
          // #endregion request

          // #region response - Status line
          // RFC 9112 section 4
          state!(response) => {
            if crlf_new!() {
              // RFC 9112 section 2.2 - Repeated
              advance!(2);
            } else if data.len() >= 5 && data[4] == b'/' && (data.starts_with(b"HTTP") || data.starts_with(b"RTSP")) {
              self.clear();
              callback!(on_protocol, 4);
              advance!(5);
              move_to!(response_version);
            } else if data.len() >= 5 {
              fail!(UNEXPECTED_CHARACTER, "Expected protocol");
            } else {
              suspend!();
            }
          }

          state!(response_version) => {
            if data.len() >= 4 {
              if DIGIT_TABLE[data[0] as usize] && data[1] == b'.' && DIGIT_TABLE[data[2] as usize] && data[3] == b' ' {
                if data[0] == b'1' && data[2] == b'1' {
                  self.version_major = 1;
                  self.version_minor = 1;
                } else if data[0] == b'2' && data[2] == b'0' {
                  self.version_major = 2;
                  self.version_minor = 0;
                } else {
                  fail!(INVALID_VERSION, "Invalid HTTP version");
                }

                callback!(on_version, 3);
                advance!(4);
                move_to!(response_status);
              } else {
                fail!(UNEXPECTED_CHARACTER, "Expected HTTP version");
              }
            } else {
              suspend!();
            }
          }

          state!(response_status) => {
            if data.len() >= 4 {
              if DIGIT_TABLE[data[0] as usize]
                & DIGIT_TABLE[data[1] as usize]
                & DIGIT_TABLE[data[2] as usize]
                & (data[3] == b' ')
              {
                self.status =
                  ((data[0] - b'0') as u32) * 100 + ((data[1] - b'0') as u32) * 10 + ((data[2] - b'0') as u32);

                callback!(on_status, 3);
                advance!(4);
                move_to!(response_reason);
              } else {
                fail!(INVALID_STATUS, "Expected HTTP response status");
              }
            } else {
              suspend!();
            }
          }

          state!(response_reason) => {
            let valid = consume_new(data, &TOKEN_VALUE_TABLE);

            if let Some(valid) = valid {
              if data.len() - valid >= 2 {
                if data[valid] == b'\r' && data[valid + 1] == b'\n' {
                  if valid > 0 {
                    callback!(on_reason, valid);
                  }

                  advance!(valid + 2);
                  move_to!(header_name);
                } else {
                  fail!(UNEXPECTED_CHARACTER, "Expected status reason");
                }
              } else {
                suspend!();
              }
            } else {
              // Entire buffer consumed, wait for more data
              break 'parser;
            }
          }

          // TODO@PI: Continue here
          // #region headers - Headers
          // RFC 9112 section.4
          state!(header_name) => {
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
                advance!(string_length!("content-length", 1));
                move_to!(header_content_length);
                parse_next!();
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
                advance!(string_length!("transfer-encoding", 1));
                move_to!(header_transfer_encoding);
                parse_next!();
              }
              case_insensitive_string!("connection:") => {
                callback!(on_header_name, string_length!("connection"));
                advance!(string_length!("connection", 1));
                move_to!(header_connection);
                parse_next!();
              }
              // RFC 9110 section 9.5
              case_insensitive_string!("trailer:") => {
                self.has_trailers = true;
                callback!(on_header_name, string_length!("trailer"));
                advance!(string_length!("trailer", 1));
                move_to!(header_value);
                parse_next!();
              }
              // RFC 9110 section 7.8
              case_insensitive_string!("upgrade:") => {
                self.has_upgrade = true;
                callback!(on_header_name, string_length!("upgrade"));
                advance!(string_length!("upgrade", 1));
                move_to!(header_value);
                parse_next!();
              }
              _ => {}
            }

            consume!(token);

            match data[consumed..] {
              [char!(':'), ..] if consumed > 0 => {
                callback!(on_header_name, consumed);
                advance!(consumed + 1);
                move_to!(header_value);
              }
              crlf!() => {
                self.continue_without_data = true;
                advance!(2);
                move_to!(headers);
              }
              otherwise!(2) => {
                fail!(UNEXPECTED_CHARACTER, "Invalid header field name character");
              }
              _ => {
                suspend!();
              }
            }
          }

          // RFC 9112 section 6.1
          state!(header_transfer_encoding) => {
            // Ignore trailing OWS
            consume!(ws);

            if consumed > 0 {
              advance!(consumed);
              parse_next!();
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
                  "The value \"chunked\" in the Transfer-Encoding header must be the last provided and can be \
                   provided only once"
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
                self.continue_without_data = true;
                advance!(consumed + 4);
                move_to!(headers);
              }
              crlf!() => {
                callback!(on_header_value, consumed);
                advance!(consumed + 2);
                move_to!(header_name);
              }
              otherwise!(2) => {
                fail!(INVALID_TRANSFER_ENCODING, "Invalid header field value character");
              }
              _ => {
                suspend!();
              }
            }
          }

          // RFC 9112 section 6.2
          state!(header_content_length) => {
            // Ignore trailing OWS
            consume!(ws);

            if consumed > 0 {
              advance!(consumed);
              parse_next!();
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
                  advance!(consumed + 2);
                  move_to!(header_name);
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
          }

          // RFC 9112 section 9.6
          state!(header_connection) => {
            // Ignore trailing OWS
            consume!(ws);

            if consumed > 0 {
              advance!(consumed);
              parse_next!();
            }

            match data {
              case_insensitive_string!("close\r\n") => {
                self.connection = CONNECTION_CLOSE;
                callback!(on_header_value, string_length!("close"));
                advance!(string_length!("close", 2));
                move_to!(header_name);
                parse_next!();
              }
              case_insensitive_string!("keep-alive\r\n") => {
                self.connection = CONNECTION_KEEPALIVE;
                callback!(on_header_value, string_length!("keep-alive"));
                advance!(string_length!("keep-alive", 2));
                move_to!(header_name);
                parse_next!();
              }
              case_insensitive_string!("upgrade\r\n") => {
                self.connection = CONNECTION_UPGRADE;
                callback!(on_header_value, string_length!("upgrade"));
                advance!(string_length!("upgrade", 2));
                move_to!(header_name);
                parse_next!();
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
                self.continue_without_data = true;
                advance!(consumed + 4);
                move_to!(headers);
              }
              crlf!() => {
                callback!(on_header_value, consumed);
                advance!(consumed + 2);
                move_to!(header_name);
              }
              otherwise!(2) => {
                fail!(UNEXPECTED_CHARACTER, "Invalid header field value character");
              }
              _ => {
                suspend!();
              }
            }
          }

          // RFC 9110 section 5.5 and 5.6
          state!(header_value) => {
            // Ignore leading OWS
            consume!(ws);

            if consumed > 0 {
              advance!(consumed);
              parse_next!();
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
                self.continue_without_data = true;
                advance!(consumed + 4);
                move_to!(headers);
              }
              crlf!() => {
                callback!(on_header_value, trimmed_consumed);
                advance!(consumed + 2);
                move_to!(header_name);
              }
              otherwise!(2) => {
                fail!(UNEXPECTED_CHARACTER, "Invalid header field value character");
              }
              _ => {
                suspend!();
              }
            }
          }

          // RFC 9110 section 9.3.6 and 7.8 - Headers have finished, check if the
          // connection must be upgraded or a body is expected.
          state!(headers) => {
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
              move_to!(tunnel);
              parse_next!();
            }

            // In case of CONNECT method
            if self.is_connect {
              callback!(on_connect);
              move_to!(tunnel);
              parse_next!();
            }

            if (method == METHOD_GET || method == METHOD_HEAD) && self.content_length > 0 {
              fail!(UNEXPECTED_CONTENT, "Unexpected content for the request (GET or HEAD)");
            }

            // RFC 9110 section 6.3
            if self.message_type == MESSAGE_TYPE_REQUEST {
              if self.has_content_length {
                if self.content_length == 0 {
                  self.continue_without_data = true;
                  move_to!(complete);
                  parse_next!();
                }
              } else if !self.has_chunked_transfer_encoding {
                self.continue_without_data = true;
                move_to!(complete);
                parse_next!();
              }
            } else {
              if (status < 200 && status != 101) || method == METHOD_HEAD || self.skip_body {
                self.continue_without_data = true;
                move_to!(complete);
                parse_next!();
              }

              if self.content_length == 0 {
                if self.has_content_length {
                  self.continue_without_data = true;
                  move_to!(complete);
                  parse_next!();
                } else if !self.has_chunked_transfer_encoding {
                  move_to!(body_with_no_length);
                  parse_next!();
                }
              } else if status == 304 {
                // RFC 9110 section 15.4.5
                self.continue_without_data = true;
                move_to!(complete);
                parse_next!();
              }
            }

            if self.content_length > 0 {
              move_to!(body_via_content_length);
              parse_next!();
            }

            if self.has_trailers && !self.has_chunked_transfer_encoding {
              fail!(
                UNEXPECTED_TRAILERS,
                "Trailers are not allowed when not using chunked transfer encoding"
              );
            }

            move_to!(chunk_length);
          }

          // #endregion headers

          // RFC 9110 section 6.4.1 - Message completed
          state!(complete) => {
            callback!(on_message_complete);
            callback!(on_reset);

            let must_close = self.connection == CONNECTION_CLOSE;

            if must_close {
              callback!(on_finish);
              move_to!(finish);
            } else {
              move_to!(start);
            }
          }

          // Return PAUSE makes this method idempotent without failing - In this state
          // all data is ignored since the connection is not in HTTP anymore
          state!(tunnel) => {
            suspend!();
          }

          // #region body via Content-Length
          // RFC 9112 section 6.2
          state!(body_via_content_length) => {
            let expected = self.remaining_content_length;
            let available_64 = available as u64;

            // Less data than what it is expected
            if available_64 < expected {
              self.remaining_content_length -= available_64;
              callback!(on_data, available);

              advance!(available);
              parse_next!();
            }

            self.remaining_content_length = 0;
            callback!(on_data, expected as usize);
            callback!(on_body);

            self.continue_without_data = true;
            advance!(expected as usize);
            move_to!(complete);
          }
          // #endregion body via Content-Length

          // RFC 9110 section 6.3 - Body with no length nor chunked encoding. This is only
          // allowed in responses.
          //
          // Note that on_body can't and will not be called here as there is no way to
          // know when the response finishes.
          state!(body_with_no_length) => {
            callback!(on_data, available);
            advance!(available);
          }

          // #region body via chunked Transfer-Encoding
          // RFC 9112 section 7.1
          state!(chunk_length) => {
            consume!(hex_digit);

            match data[consumed..] {
              [char!(';'), ..] if consumed > 0 => {
                // Parse the length as integer
                if let Ok(length) = u64::from_str_radix(unsafe { str::from_utf8_unchecked(&data[..consumed]) }, 16) {
                  callback!(on_chunk_length, consumed);
                  self.chunk_size = length;
                  self.remaining_chunk_size = length;
                  advance!(consumed + 1);
                  move_to!(chunk_extension_name);
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
                  advance!(consumed + 2);
                  move_to!(chunk_data);
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
          }

          state!(chunk_extension_name) => {
            consume!(token);

            if consumed == 0 {
              fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension name character");
            }

            match data[consumed..] {
              [char!('='), ..] => {
                callback!(on_chunk_extension_name, consumed);
                advance!(consumed + 1);
                move_to!(chunk_extension_value);
              }
              [char!(';'), ..] => {
                callback!(on_chunk_extension_name, consumed);
                advance!(consumed + 1);
                move_to!(chunk_extension_name);
              }
              crlf!() => {
                callback!(on_chunk_extension_name, consumed);

                self.continue_without_data = true;
                advance!(consumed + 2);
                move_to!(chunk_data);
              }
              otherwise!(2) => {
                fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension name character");
              }
              _ => {
                suspend!();
              }
            }
          }

          state!(chunk_extension_value) => {
            if data[0] == char!('"') {
              advance!(1);
              move_to!(chunk_extension_quoted_value);
              parse_next!();
            }

            consume!(token);

            if consumed == 0 {
              fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension value character");
            }

            match data[consumed..] {
              [char!(';'), ..] => {
                callback!(on_chunk_extension_value, consumed);
                advance!(consumed + 1);
                move_to!(chunk_extension_name);
              }
              crlf!() => {
                callback!(on_chunk_extension_value, consumed);
                self.continue_without_data = true;
                advance!(consumed + 2);
                move_to!(chunk_data);
              }
              otherwise!(2) => {
                fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension value character");
              }
              _ => {
                suspend!();
              }
            }
          }

          // RFC 9110 section 5.6.4
          state!(chunk_extension_quoted_value) => {
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
                advance!(consumed + 2);
                move_to!(chunk_data);
              }
              [char!(';'), ..] => {
                self.continue_without_data = true;
                callback!(on_chunk_extension_value, consumed - 1);
                advance!(consumed + 1);
                move_to!(chunk_extension_name);
              }
              otherwise!(3) => {
                fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension quoted value character");
              }
              _ => {
                suspend!();
              }
            }
          }

          state!(chunk_data) => {
            // When receiving the last chunk
            if self.chunk_size == 0 {
              callback!(on_chunk);
              callback!(on_body);

              if self.has_trailers {
                move_to!(trailer_name);
              } else {
                move_to!(crlf_after_last_chunk);
              }

              parse_next!();
            }

            let expected = self.remaining_chunk_size;
            let available_64 = available as u64;

            // Less data than what it is expected for this chunk
            if available_64 < expected {
              self.remaining_chunk_size -= available_64;

              callback!(on_chunk);
              callback!(on_data, available);

              advance!(available);
              parse_next!();
            }

            self.remaining_chunk_size = 0;

            callback!(on_chunk);
            callback!(on_data, expected as usize);

            advance!(expected as usize);
            move_to!(chunk_end);
          }

          state!(chunk_end) => {
            match data {
              crlf!() => {
                self.chunk_size = 0;
                self.remaining_chunk_size = 0;
                advance!(2);
                move_to!(chunk_length);
              }
              otherwise!(2) => {
                fail!(UNEXPECTED_CHARACTER, "Unexpected character after chunk data");
              }
              _ => {
                suspend!();
              }
            }
          }

          state!(crlf_after_last_chunk) => {
            match data {
              crlf!() => {
                self.continue_without_data = true;
                advance!(2);
                move_to!(complete);
              }
              otherwise!(2) => {
                fail!(UNEXPECTED_CHARACTER, "Expected CRLF after the last chunk");
              }
              _ => {
                suspend!();
              }
            }
          }

          // #endregion body via chunked Transfer-Encoding

          // #region trailers - Trailers
          // RFC 9112 section 7.1.2
          state!(trailer_name) => {
            consume!(token);

            match data[consumed..] {
              [char!(':'), ..] if consumed > 0 => {
                callback!(on_trailer_name, consumed);
                advance!(consumed + 1);
                move_to!(trailer_value);
              }
              crlf!() => {
                self.continue_without_data = true;
                advance!(2);
                move_to!(trailers);
              }
              otherwise!(2) => {
                fail!(UNEXPECTED_CHARACTER, "Invalid trailer field name character");
              }
              _ => {
                suspend!();
              }
            }
          }

          state!(trailer_value) => {
            // Ignore leading OWS
            consume!(ws);

            if consumed > 0 {
              advance!(consumed);
              parse_next!();
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
                self.continue_without_data = true;
                advance!(consumed + 4);
                move_to!(trailers);
              }
              crlf!() => {
                callback!(on_trailer_value, trimmed_consumed);
                advance!(consumed + 2);
                move_to!(trailer_name);
              }
              otherwise!(2) => {
                fail!(UNEXPECTED_CHARACTER, "Invalid trailer field value character");
              }
              _ => {
                suspend!();
              }
            }
          }

          state!(trailers) => {
            callback!(on_trailers);
            self.continue_without_data = true;
            move_to!(complete);
          }
          _ => {
            fail!(UNEXPECTED_STATE, "Invalid state");
          }
        }
        // #endregion trailers
      }

      // Update the parser position
      if advanced > 0 {
        self.position += advanced;
        data = &data[advanced..];
        available -= advanced;

        #[cfg(any(debug_assertions, feature = "debug"))]
        {
          eprintln!(
            "[milo::debug] loop before processing: position={}, advanced={}, available={}, continue_without_data={}",
            self.position, advanced, available, self.continue_without_data
          );
        }
      }

      // Notify the status change
      #[cfg(any(debug_assertions, feature = "debug"))]
      if previous_state != self.state {
        callback!(on_state_change);
        previous_state = self.state;
      }

      // Show the duration of the operation
      #[cfg(any(debug_assertions, feature = "debug"))]
      {
        let duration = Instant::now().duration_since(last).as_nanos();

        if duration > 0 {
          eprintln!(
            "[milo::debug] loop iteration ({:?}) completed in {} ns",
            self.state_str(),
            duration
          );
        }

        last = Instant::now();
      }
    }

    #[cfg(any(debug_assertions, feature = "debug"))]
    eprintln!("[milo::debug] loop exit");

    let consumed = self.position;
    self.parsed += consumed as u64;

    if self.manage_unconsumed {
      unsafe {
        // Drop any previous retained data
        if unconsumed_len > 0 {
          let _ = from_raw_parts(self.unconsumed, unconsumed_len);
        }

        // If less bytes were consumed than requested, copy the unconsumed portion in
        // the self.for the next iteration
        if consumed < limit {
          let (ptr, len, _) = data.to_vec().into_raw_parts();

          self.unconsumed = ptr;
          self.unconsumed_len = len;
        } else {
          self.unconsumed = ptr::null();
          self.unconsumed_len = 0;
        }
      }
    }

    #[cfg(any(debug_assertions, feature = "debug"))]
    {
      let duration = Instant::now().duration_since(start).as_nanos();

      if duration > 0 {
        eprintln!(
          "[milo::debug] parse ({:?}, consumed {} of {}) completed in {} ns",
          self.state_str(),
          consumed,
          limit,
          duration
        );
      }
    }

    // Return the number of consumed bytes
    consumed
  }
}
