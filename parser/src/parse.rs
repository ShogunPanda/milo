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
use std::string;
#[cfg(all(not(target_family = "wasm"), any(debug_assertions, feature = "debug")))]
use std::time::Instant;

use milo_macros::*;

use crate::*;

enum StateResult {
  Continue,
  Suspend,
  Stop,
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

    #[cfg(all(not(target_family = "wasm"), any(debug_assertions, feature = "debug")))]
    let mut last = Instant::now();

    #[cfg(all(not(target_family = "wasm"), any(debug_assertions, feature = "debug")))]
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
    if self.debug {
      eprintln!("[milo::debug] loop enter");
    }

    // Until there is data or there is a request to continue
    'parser: while parsing && (!self.paused) && (available != 0 || self.continue_without_data) {
      #[cfg(any(debug_assertions, feature = "debug"))]
      if self.debug {
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
          STATE_START => {
            match self.mode {
              MESSAGE_TYPE_AUTODETECT => {
                move_to!(autodetect);
              }
              MESSAGE_TYPE_REQUEST => {
                self.message_type = MESSAGE_TYPE_REQUEST;
                callback!(on_request);
                callback!(on_message_start);
                move_to!(request_line);
              }
              MESSAGE_TYPE_RESPONSE => {
                self.message_type = MESSAGE_TYPE_RESPONSE;
                callback!(on_response);
                callback!(on_message_start);
                move_to!(status_line);
              }
              _ => {
                fail!(UNEXPECTED_CHARACTER, "Invalid mode");
              }
            }
          }

          // If the parser has finished and it receives more data, error
          STATE_FINISH => {
            fail!(UNEXPECTED_CHARACTER, "Unexpected data");
          }

          // The error state is a no-op
          STATE_ERROR => {
            suspend!();
          }

          // Autodetect if there is a HTTP/RTSP method or a response
          STATE_AUTODETECT => {
            if data.len() >= 5 && data[4] == b'/' && (data.starts_with(b"HTTP") || data.starts_with(b"RTSP")) {
              self.message_type = MESSAGE_TYPE_RESPONSE;
              callback!(on_response);
              callback!(on_message_start);
              move_to!(status_line);
            } else if data.len() >= 2 && data.starts_with(b"\r\n") {
              // RFC 9112 section 2.2
              advance!(2);
            } else {
              // For performance reason, we assume it's a request so we don't lookup the
              // method twice
              self.message_type = MESSAGE_TYPE_REQUEST;
              callback!(on_request);
              callback!(on_message_start);
              move_to!(request_line);
            }
          }

          STATE_REQUEST_LINE => {
            match self.find_cr(data, available) {
              // // RFC 9112 section 3
              Some(cr) => {
                match self.ensure_valid_line(data, cr, available) {
                  StateResult::Continue => {}
                  StateResult::Suspend => {
                    suspend!();
                  }
                  StateResult::Stop => {
                    stop!();
                  }
                }

                // RFC 9112 section 2.2 - Repeated
                if cr == 0 {
                  advance!(2);
                  next!();
                } else if cr < 14
                // Length of "GET / HTTP/1.1"
                {
                  fail!(UNEXPECTED_CHARACTER, "Request line too short");
                }

                // The line is potentially valid, clear the parser
                self.clear();

                // RFC 9112 section 3.1
                let method_start = 0;
                let method_end = match self.find_char(data, method_start, cr, b' ') {
                  Some(index) if index > method_start => index,
                  _ => {
                    fail!(UNEXPECTED_CHARACTER, "Expected space after method");
                  }
                };

                // RFC 9112 section 3.2
                let url_start = method_end + 1;
                let url_end = match self.find_char(data, url_start, cr, b' ') {
                  Some(index) if index > url_start => index,
                  _ => {
                    fail!(UNEXPECTED_CHARACTER, "Expected space after URL");
                  }
                };

                // RFC 9112 section 2.3
                let protocol_start = url_end + 1;
                let protocol_end = match self.find_char(data, protocol_start, cr, b'/') {
                  Some(index) if index > protocol_start => index,
                  _ => {
                    fail!(UNEXPECTED_CHARACTER, "Expected / after the protocol name");
                  }
                };

                if let Some(&method) = METHODS.get(&data[method_start..method_end]) {
                  self.method = method;

                  if method == METHOD_CONNECT {
                    self.is_connect = true;
                  }
                } else {
                  fail!(UNEXPECTED_CHARACTER, "Invalid method");
                }

                if let StateResult::Stop = self.validate(data, &URL_TABLE, url_start, url_end, "Invalid URL character")
                {
                  stop!();
                }

                let protocol = &data[protocol_start..protocol_end];
                if protocol_end - protocol_start != 4 || (protocol != b"HTTP" && protocol != b"RTSP") {
                  fail!(UNEXPECTED_CHARACTER, "Invalid protocol name");
                }

                let version_start = protocol_end + 1;
                let version_len = cr - version_start;

                if version_len != 3
                  || !DIGIT_TABLE[data[version_start] as usize]
                  || data[version_start + 1] != b'.'
                  || !DIGIT_TABLE[data[version_start + 2] as usize]
                {
                  fail!(UNEXPECTED_CHARACTER, "Invalid protocol version");
                }

                if data[version_start] == b'1' && data[version_start + 2] == b'1' {
                  self.version_major = 1;
                  self.version_minor = 1;
                } else if data[version_start] == b'2' && data[version_start + 2] == b'0' {
                  self.version_major = 2;
                  self.version_minor = 0;

                  if self.method == METHOD_PRI {
                    fail!(UNSUPPORTED_HTTP_VERSION, "HTTP/2.0 is not supported");
                  }
                } else {
                  fail!(INVALID_VERSION, "Invalid HTTP version");
                }

                callback!(on_method, method_start, method_end - method_start);
                callback!(on_url, url_start, url_end - url_start);
                callback!(on_protocol, protocol_start, protocol_end - protocol_start);
                callback!(on_version, version_start, 3);

                advance!(cr + 2);
                move_to!(header);
              }
              None => {
                if available >= self.max_start_line_length {
                  fail!(UNEXPECTED_CHARACTER, "Request line too long");
                } else {
                  suspend!();
                }
              }
            }
          }

          // RFC 9112 section 4
          STATE_STATUS_LINE => {
            match self.find_cr(data, available) {
              Some(cr) => {
                match self.ensure_valid_line(data, cr, available) {
                  StateResult::Continue => {}
                  StateResult::Suspend => {
                    suspend!();
                  }
                  StateResult::Stop => {
                    stop!();
                  }
                }

                // RFC 9112 section 2.2 - Repeated
                if cr == 0 {
                  advance!(2);
                  next!();
                } else if cr < 13
                // Length of "HTTP/1.1 200 "
                {
                  fail!(UNEXPECTED_CHARACTER, "Status line too short");
                }

                // The line is potentially valid, clear the parser
                self.clear();

                let protocol_start = 0;
                let protocol_end = 4;
                if data[protocol_end] != b'/' {
                  fail!(UNEXPECTED_CHARACTER, "Expected protocol");
                }

                if &data[protocol_start..protocol_end] != b"HTTP" && &data[protocol_start..protocol_end] != b"RTSP" {
                  fail!(UNEXPECTED_CHARACTER, "Invalid protocol");
                }

                let version_start = protocol_end + 1;
                let version_end = match self.find_char(data, version_start, cr, b' ') {
                  Some(index) if index > version_start => index,
                  _ => {
                    fail!(UNEXPECTED_CHARACTER, "Expected space after protocol");
                  }
                };

                if version_end - version_start != 3
                  || !DIGIT_TABLE[data[version_start] as usize]
                  || data[version_start + 1] != b'.'
                  || !DIGIT_TABLE[data[version_start + 2] as usize]
                {
                  fail!(UNEXPECTED_CHARACTER, "Expected HTTP version");
                }

                if data[version_start] == b'1' && data[version_start + 2] == b'1' {
                  self.version_major = 1;
                  self.version_minor = 1;
                } else if data[version_start] == b'2' && data[version_start + 2] == b'0' {
                  self.version_major = 2;
                  self.version_minor = 0;
                } else {
                  fail!(INVALID_VERSION, "Invalid HTTP version");
                }

                let status_start = version_end + 1;
                // Even if the reason is empty, there must be at least a space after the status
                // code. So three digits plus a space
                let status_end = version_end + 5;
                if status_end > cr {
                  fail!(INVALID_STATUS, "Expected HTTP response status");
                }

                if !DIGIT_TABLE[data[status_start] as usize]
                  || !DIGIT_TABLE[data[status_start + 1] as usize]
                  || !DIGIT_TABLE[data[status_start + 2] as usize]
                {
                  fail!(INVALID_STATUS, "Invalid HTTP response status");
                }

                if data[status_start + 3] != b' ' {
                  fail!(INVALID_STATUS, "Expected a space after HTTP response status");
                }

                let reason_start = status_start + 4;
                let reason_end = cr;
                if let StateResult::Stop = self.validate(
                  data,
                  &TOKEN_VALUE_TABLE,
                  reason_start,
                  reason_end,
                  "Invalid status reason character",
                ) {
                  stop!();
                }

                self.status = ((data[status_start] - b'0') as u32) * 100
                  + ((data[status_start + 1] - b'0') as u32) * 10
                  + (data[status_start + 2] - b'0') as u32;

                callback!(on_protocol, protocol_start, 4);
                callback!(on_version, version_start, 3);
                callback!(on_status, status_start, 3);
                if reason_end > reason_start {
                  callback!(on_reason, reason_start, reason_end - reason_start);
                }

                advance!(cr + 2);
                move_to!(header);
              }
              None => {
                if available >= self.max_start_line_length {
                  fail!(UNEXPECTED_CHARACTER, "Status line too long");
                } else {
                  suspend!();
                }
              }
            }
          }

          STATE_HEADER => {
            match self.find_cr(data, available) {
              Some(cr) => {
                match self.ensure_valid_line(data, cr, available) {
                  StateResult::Continue => {}
                  StateResult::Suspend => {
                    suspend!();
                  }
                  StateResult::Stop => {
                    stop!();
                  }
                }

                // No more headers or no headers at all, move to the headers state
                if cr == 0 {
                  self.continue_without_data = true;
                  advance!(2);
                  move_to!(body_decision);
                  next!();
                }

                // RFC 9112 section.4
                // RFC 9110 section 5.5 and 5.6
                let header_name_start = 0;
                let header_name_end = match self.find_char(data, header_name_start, cr, b':') {
                  Some(index) if index > header_name_start => index,
                  _ => {
                    fail!(UNEXPECTED_CHARACTER, "Invalid header field name character");
                  }
                };
                let header_name_len = header_name_end - header_name_start;

                let mut header_value_start = header_name_end + 1;
                let mut header_value_end = cr;

                let status = self.status;
                match &data[header_name_start..header_name_end] {
                  // RFC 9112 section 6.2
                  case_insensitive_string!("content-length") => {
                    // It just matched a prefix, invalid header
                    if header_name_len > 14 {
                      fail!(UNEXPECTED_CHARACTER, "Invalid header field name");
                    } else if self.has_chunked_transfer_encoding {
                      fail!(
                        UNEXPECTED_CONTENT_LENGTH,
                        "Unexpected Content-Length header when Transfer-Encoding header is present"
                      );
                    } else if status == 204 || status / 100 == 1 {
                      fail!(
                        UNEXPECTED_CONTENT_LENGTH,
                        "Unexpected Content-Length header for a response with status 204 or 1xx"
                      );
                    } else if self.has_content_length {
                      fail!(INVALID_CONTENT_LENGTH, "Invalid duplicate Content-Length header");
                    }

                    if let StateResult::Stop = self.strip_ows(
                      data,
                      &mut header_value_start,
                      &mut header_value_end,
                      "Expected Content-Length header value",
                    ) {
                      stop!();
                    }

                    let mut i = header_value_start;
                    let mut content_length = 0u64;

                    if header_value_end - header_value_start > 19 {
                      // 19 digits are enough to represent 2^63-1, which is the maximum value we allow
                      // for
                      fail!(INVALID_CONTENT_LENGTH, "Invalid Content-Length header");
                    }

                    while i < header_value_end {
                      let current = data[i];
                      if !DIGIT_TABLE[current as usize] {
                        fail!(INVALID_CONTENT_LENGTH, "Invalid Content-Length header");
                      }

                      content_length = content_length * 10 + (current - b'0') as u64;
                      i += 1;
                    }

                    self.has_content_length = true;
                    self.content_length = content_length;
                    self.remaining_content_length = content_length;
                  }
                  // RFC 9112 section 6.1
                  case_insensitive_string!("transfer-encoding") => {
                    // It just matched a prefix, invalid header
                    if header_name_len > 17 {
                      fail!(UNEXPECTED_CHARACTER, "Invalid header field name");
                    } else if self.has_content_length {
                      fail!(
                        UNEXPECTED_TRANSFER_ENCODING,
                        "Unexpected Transfer-Encoding header when Content-Length header is present"
                      );
                    } else if status == 304 {
                      fail!(
                        UNEXPECTED_TRANSFER_ENCODING,
                        "Unexpected Transfer-Encoding header for a response with status 304"
                      );
                    }

                    if let StateResult::Stop = self.strip_ows(
                      data,
                      &mut header_value_start,
                      &mut header_value_end,
                      "Expected Transfer-Encoding header value",
                    ) {
                      stop!();
                    }

                    if &data[header_value_start..header_value_end] == b"chunked" {
                      // If this is true, it means the Transfer-Encoding header was specified more
                      // than once. This is the second repetition and therefore, the previous one is
                      // no longer the last one, making it invalid.
                      if self.has_chunked_transfer_encoding {
                        fail!(
                          INVALID_TRANSFER_ENCODING,
                          "The value \"chunked\" in the Transfer-Encoding header must be the last provided and can be \
                           provided only once"
                        );
                      }

                      self.has_chunked_transfer_encoding = true;
                    } else {
                      let mut token_start = header_value_start;
                      loop {
                        while token_start < header_value_end && WS_TABLE[data[token_start] as usize] {
                          token_start += 1;
                        }

                        if token_start == header_value_end {
                          break;
                        }

                        let token_end_raw = match self.find_char(data, token_start, header_value_end, b',') {
                          Some(comma) => comma,
                          None => header_value_end,
                        };
                        let mut token_end = token_end_raw;

                        if let StateResult::Stop = self.strip_ows(
                          data,
                          &mut token_start,
                          &mut token_end,
                          "Expected Transfer-Encoding header value",
                        ) {
                          stop!();
                        }

                        if let case_insensitive_string!("chunked") = data[token_start..token_end] {
                          // If this is true, it means the Transfer-Encoding header was specified more
                          // than once. This is the second repetition and therefore, the previous one is
                          // no longer the last one, making it invalid.
                          if self.has_chunked_transfer_encoding {
                            fail!(
                              INVALID_TRANSFER_ENCODING,
                              "The value \"chunked\" in the Transfer-Encoding header must be the last provided and \
                               can be provided only once"
                            );
                          }

                          self.has_chunked_transfer_encoding = true;
                        } else {
                          if self.has_chunked_transfer_encoding {
                            // Any other value when chunked was already specified is invalid as the previous
                            // chunked would not be the last one anymore
                            fail!(
                              INVALID_TRANSFER_ENCODING,
                              "The value \"chunked\" in the Transfer-Encoding header must be the last provided"
                            );
                          }

                          if let StateResult::Stop = self.validate(
                            data,
                            &TOKEN_TABLE,
                            token_start,
                            token_end,
                            "Invalid Transfer-Encoding header value character",
                          ) {
                            stop!();
                          }
                        }

                        if token_end_raw == header_value_end {
                          break;
                        } else {
                          token_start = token_end_raw + 1;
                        }
                      }
                    }
                  }
                  // RFC 9112 section 9.6
                  case_insensitive_string!("connection") => {
                    // It just matched a prefix, invalid header
                    if header_name_len > 10 {
                      fail!(UNEXPECTED_CHARACTER, "Invalid header field name");
                    }

                    if let StateResult::Stop = self.strip_ows(
                      data,
                      &mut header_value_start,
                      &mut header_value_end,
                      "Expected Connection header value",
                    ) {
                      stop!();
                    }

                    match data[header_value_start..header_value_end] {
                      case_insensitive_string!("close") => {
                        self.connection = CONNECTION_CLOSE;
                      }
                      case_insensitive_string!("keep-alive") => {
                        self.connection = CONNECTION_KEEPALIVE;
                      }
                      case_insensitive_string!("upgrade") => {
                        self.connection = CONNECTION_UPGRADE;
                      }
                      _ => {}
                    }
                  }
                  case_insensitive_string!("trailer") => {
                    // It just matched a prefix, invalid header
                    if header_name_len > 7 {
                      fail!(UNEXPECTED_CHARACTER, "Invalid header field name");
                    }

                    self.has_trailers = true;

                    if let StateResult::Stop = self.strip_ows(
                      data,
                      &mut header_value_start,
                      &mut header_value_end,
                      "Expected Trailer header value",
                    ) {
                      stop!();
                    }
                  }
                  case_insensitive_string!("upgrade") => {
                    // It just matched a prefix, invalid header
                    if header_name_len > 7 {
                      fail!(UNEXPECTED_CHARACTER, "Invalid header field name");
                    }

                    self.has_upgrade = true;

                    if let StateResult::Stop =
                      self.strip_ows_allowing_empty(data, &mut header_value_start, &mut header_value_end)
                    {
                      stop!();
                    }
                  }
                  _ => {
                    if let StateResult::Stop = self.validate(
                      data,
                      &TOKEN_TABLE,
                      header_name_start,
                      header_name_end,
                      "Invalid header field name character",
                    ) {
                      stop!();
                    }

                    if let StateResult::Stop =
                      self.strip_ows_allowing_empty(data, &mut header_value_start, &mut header_value_end)
                    {
                      stop!();
                    }
                  }
                }

                callback!(on_header_name, header_name_start, header_name_end - header_name_start);
                callback!(
                  on_header_value,
                  header_value_start,
                  header_value_end - header_value_start
                );

                advance!(cr + 2);
              }
              None => {
                if available >= self.max_header_length {
                  fail!(UNEXPECTED_CHARACTER, "Header line too long");
                } else {
                  suspend!();
                }
              }
            }
          }

          // RFC 9110 section 9.3.6 and 7.8 - Headers have finished, check if the
          // connection must be upgraded or a body is expected.
          STATE_BODY_DECISION => {
            callback!(on_headers);

            let method = self.method;
            let status = self.status;

            if self.has_upgrade {
              if self.connection != CONNECTION_UPGRADE {
                fail!(
                  MISSING_CONNECTION_UPGRADE,
                  "Missing Connection header set to \"upgrade\" when using the Upgrade header"
                );
              }
            } else if self.has_trailers && !self.has_chunked_transfer_encoding {
              {
                fail!(
                  UNEXPECTED_TRAILERS,
                  "Trailers are not allowed when not using chunked transfer encoding"
                );
              }
            } else if (method == METHOD_GET || method == METHOD_HEAD) && self.content_length > 0 {
              fail!(UNEXPECTED_CONTENT, "Unexpected content for the request (GET or HEAD)");
            }

            // In case of Connection: Upgrade or a CONNECT method
            if self.is_connect {
              // In case of CONNECT method
              callback!(on_connect);
              move_to!(tunnel);
            } else if self.has_upgrade {
              callback!(on_upgrade);
              move_to!(tunnel);
            } else if self.message_type == MESSAGE_TYPE_REQUEST {
              // RFC 9110 section 6.3
              if self.has_content_length {
                if self.content_length == 0 {
                  self.continue_without_data = true;
                  self.complete(0);
                } else {
                  move_to!(body_via_content_length);
                }
              } else if !self.has_chunked_transfer_encoding {
                self.continue_without_data = true;
                self.complete(0);
              } else {
                move_to!(chunk_header);
              }
            } else {
              // Response
              // RFC 9110 section 15.4.5
              if (status < 200 && status != 101) || method == METHOD_HEAD || self.skip_body || status == 304 {
                self.continue_without_data = true;
                self.complete(0);
              } else if self.has_content_length {
                if self.content_length == 0 {
                  self.continue_without_data = true;
                  self.complete(0);
                } else {
                  move_to!(body_via_content_length);
                }
              } else if self.has_chunked_transfer_encoding {
                move_to!(chunk_header);
              } else {
                move_to!(body_with_no_length);
              }
            }
          }

          // RFC 9112 section 6.2
          STATE_BODY_VIA_CONTENT_LENGTH => {
            let expected = self.remaining_content_length;
            let available_64 = available as u64;

            // Less data than what it is expected
            if available_64 < expected {
              self.remaining_content_length -= available_64;

              callback!(on_data, 0, available);
              advance!(available);
            } else {
              self.remaining_content_length = 0;

              callback!(on_data, 0, expected as usize);
              callback!(on_body, expected as usize, 0);

              self.continue_without_data = true;

              advance!(expected as usize);
              self.complete(expected as usize);
            }
          }

          // RFC 9110 section 6.3 - Body with no length nor chunked encoding.
          // This is only allowed in responses.
          // Note that on_body can't and will not be called here as there is no way to
          // know when the response finishes.
          STATE_BODY_WITH_NO_LENGTH => {
            callback!(on_data, 0, available);
            advance!(available);
          }

          // RFC 9112 section 7.1
          STATE_CHUNK_HEADER => {
            match self.find_cr(data, available) {
              Some(cr) => {
                match self.ensure_valid_line(data, cr, available) {
                  StateResult::Continue => {}
                  StateResult::Suspend => {
                    suspend!();
                  }
                  StateResult::Stop => {
                    stop!();
                  }
                }

                let chunk_length_start = 0;
                // Note, the character is optional since chunk extensions are not required
                let chunk_length_end = match self.find_char(data, chunk_length_start, cr, b';') {
                  Some(index) => index,
                  None => cr,
                };

                if chunk_length_end == 0 {
                  fail!(UNEXPECTED_CHARACTER, "Invalid chunk length character");
                } else if chunk_length_end - chunk_length_start > 16 {
                  fail!(INVALID_CHUNK_SIZE, "Invalid chunk length size");
                }

                let mut i = chunk_length_start;
                let mut chunk_length = 0u64;
                while i < chunk_length_end {
                  let b = data[i];

                  let hex = if b >= b'0' && b <= b'9' {
                    (b - b'0') as u64
                  } else if b >= b'a' && b <= b'f' {
                    (b - b'a' + 10) as u64
                  } else if b >= b'A' && b <= b'F' {
                    (b - b'A' + 10) as u64
                  } else {
                    fail!(UNEXPECTED_CHARACTER, "Invalid chunk length character");
                  };

                  chunk_length = chunk_length * 16 + hex;
                  i += 1;
                }

                self.chunk_size = chunk_length;
                self.remaining_chunk_size = chunk_length;

                callback!(
                  on_chunk_length,
                  chunk_length_start,
                  chunk_length_end - chunk_length_start
                );

                // There are extensions
                if chunk_length_end < cr {
                  advance!(chunk_length_end + 1);
                  move_to!(chunk_extensions);
                } else {
                  self.continue_without_data = true;
                  advance!(cr + 2);

                  if self.chunk_size == 0 {
                    callback!(on_chunk, 3, 0);
                    callback!(on_body, 3, 0);
                    move_to!(trailer);
                  } else {
                    move_to!(chunk_data);
                  }
                }
              }
              None => {
                if available >= self.max_header_length {
                  fail!(UNEXPECTED_CHARACTER, "Chunk header too long");
                } else {
                  suspend!();
                }
              }
            }
          }

          STATE_CHUNK_EXTENSIONS => {
            match self.find_cr(data, available) {
              Some(cr) => {
                match self.ensure_valid_line(data, cr, available) {
                  StateResult::Continue => {}
                  StateResult::Suspend => {
                    suspend!();
                  }
                  StateResult::Stop => {
                    stop!();
                  }
                }

                let mut name_start = 0;
                // Find the first between = or ;
                let name_end_raw = self.find_char2(data, name_start, cr, b'=', b';').unwrap_or(cr);
                let mut name_end = name_end_raw;

                if let StateResult::Stop =
                  self.strip_ows(data, &mut name_start, &mut name_end, "Expected chunk extension name")
                {
                  stop!();
                }

                if let StateResult::Stop = self.validate(
                  data,
                  &TOKEN_TABLE,
                  name_start,
                  name_end,
                  "Invalid chunk extension name character",
                ) {
                  stop!();
                }

                // No value
                if name_end == cr || data[name_end_raw] == b';' {
                  callback!(on_chunk_extension_name, name_start, name_end - name_start);

                  if name_end_raw == cr {
                    advance!(cr + 2);

                    if self.chunk_size == 0 {
                      callback!(on_body);
                      move_to!(trailer);
                    } else {
                      move_to!(chunk_data);
                    }
                  } else {
                    advance!(name_end_raw + 1);
                    move_to!(chunk_extensions);
                  }
                } else {
                  // Get the value
                  let mut value_start = name_end_raw + 1;
                  let mut value_end: usize;
                  let next_extension: usize;

                  // Strip OWS before the value
                  while value_start < cr && WS_TABLE[data[value_start] as usize] {
                    value_start += 1;
                  }

                  if value_start == cr {
                    fail!(UNEXPECTED_CHARACTER, "Expected chunk extension value");
                  }

                  // Quoted string
                  // RFC 9110 section 5.6.4
                  if data[value_start] == b'"' {
                    value_start += 1;
                    let mut quote_start = value_start;

                    loop {
                      match self.find_char(data, quote_start, cr, b'"') {
                        Some(index) => {
                          // Count consecutive backslashes immediately before the quote
                          let mut backslash_count = 0usize;
                          let mut i = index;

                          while i > quote_start && data[i - 1] == b'\\' {
                            backslash_count += 1;
                            i -= 1;
                          }

                          if backslash_count % 2 == 0 {
                            // quote is not escaped
                            value_end = index;
                            break;
                          } else {
                            // quote is escaped, continue searching after it
                            quote_start = index + 1;
                          }
                        }
                        None => {
                          fail!(UNEXPECTED_CHARACTER, "Expected closing quote for chunk extension value");
                        }
                      };
                    }

                    if let StateResult::Stop = self.validate(
                      data,
                      &TOKEN_VALUE_QUOTED_TABLE,
                      value_start,
                      value_end,
                      "Invalid chunk extension quoted value character",
                    ) {
                      stop!();
                    }

                    next_extension = value_end + 1;
                  } else {
                    value_end = self.find_char(data, value_start, cr, b';').unwrap_or(cr);
                    next_extension = if value_end == cr { cr } else { value_end + 1 };

                    if let StateResult::Stop =
                      self.strip_ows(data, &mut value_start, &mut value_end, "Expected chunk extension value")
                    {
                      stop!();
                    }

                    if let StateResult::Stop = self.validate(
                      data,
                      &TOKEN_TABLE,
                      value_start,
                      value_end,
                      "Invalid chunk extension value character",
                    ) {
                      stop!();
                    }
                  }

                  callback!(on_chunk_extension_name, name_start, name_end - name_start);
                  callback!(on_chunk_extension_value, value_start, value_end - value_start);

                  let next_semicolon = self.find_char(data, next_extension, cr, b';').unwrap_or(cr);

                  let mut i = next_extension;
                  while i < next_semicolon {
                    if !WS_TABLE[data[i] as usize] {
                      fail!(UNEXPECTED_CHARACTER, "Invalid chunk extension character after value");
                    }
                    i += 1;
                  }

                  if next_semicolon < cr {
                    advance!(next_semicolon + 1);
                  } else {
                    advance!(cr + 2);

                    if self.chunk_size == 0 {
                      callback!(on_body);
                      move_to!(trailer);
                    } else {
                      move_to!(chunk_data);
                    }
                  }
                }
              }
              None => {
                // Given in chunk_header we already validated this, this should not happen.
                if available >= self.max_header_length {
                  fail!(UNEXPECTED_CHARACTER, "Chunk header too long");
                } else {
                  suspend!();
                }
              }
            }
          }

          STATE_CHUNK_DATA => {
            let expected = self.remaining_chunk_size;
            let available_64 = available as u64;

            // No more data for this chunk, just wait for the CRLF
            if expected == 0 {
              if available < 2 {
                suspend!();
              } else if data[0] != b'\r' || data[1] != b'\n' {
                fail!(UNEXPECTED_CHARACTER, "Expected CRLF after chunk data");
              } else {
                advance!(2);
                move_to!(chunk_header);
              }
            } else if available_64 < expected {
              // Less data than what it is expected for this chunk
              self.remaining_chunk_size -= available_64;

              callback!(on_chunk, 0, available);
              callback!(on_data, 0, available);

              advance!(available);
            } else {
              self.remaining_chunk_size = 0;

              callback!(on_chunk, 0, expected as usize);
              callback!(on_data, 0, expected as usize);

              advance!(expected as usize);
            }
          }

          // RFC 9112 section 7.1.2
          STATE_TRAILER => {
            match self.find_cr(data, available) {
              Some(cr) => {
                match self.ensure_valid_line(data, cr, available) {
                  StateResult::Continue => {}
                  StateResult::Suspend => {
                    suspend!();
                  }
                  StateResult::Stop => {
                    stop!();
                  }
                }

                // No more trailers or no trailers at all, message completed
                if cr == 0 {
                  callback!(on_trailers, 2, 0);
                  self.continue_without_data = true;
                  advance!(2);
                  self.complete(2);
                  next!();
                }

                let trailer_name_start = 0;
                let trailer_name_end = match self.find_char(data, trailer_name_start, cr, b':') {
                  Some(index) if index > trailer_name_start => index,
                  _ => {
                    fail!(UNEXPECTED_CHARACTER, "Invalid trailer field name character");
                  }
                };

                let mut trailer_value_start = trailer_name_end + 1;
                let mut trailer_value_end = cr;
                if let StateResult::Stop =
                  self.strip_ows_allowing_empty(data, &mut trailer_value_start, &mut trailer_value_end)
                {
                  stop!();
                }

                // Validate
                if let StateResult::Stop = self.validate(
                  data,
                  &TOKEN_TABLE,
                  trailer_name_start,
                  trailer_name_end,
                  "Invalid trailer field name character",
                ) {
                  stop!();
                }

                if let StateResult::Stop = self.validate_allowing_empty(
                  data,
                  &TOKEN_VALUE_TABLE,
                  trailer_value_start,
                  trailer_value_end,
                  "Invalid trailer field value character",
                ) {
                  stop!();
                }

                callback!(
                  on_trailer_name,
                  trailer_name_start,
                  trailer_name_end - trailer_name_start
                );
                callback!(
                  on_trailer_value,
                  trailer_value_start,
                  trailer_value_end - trailer_value_start
                );
                advance!(cr + 2);
              }
              None => {
                if available >= self.max_header_length {
                  fail!(UNEXPECTED_CHARACTER, "Trailer line too long");
                } else {
                  suspend!();
                }
              }
            }
          }

          // Return PAUSE makes this method idempotent without failing - In this state
          // all data is ignored since the connection is not in HTTP anymore
          STATE_TUNNEL => {
            suspend!();
          }

          _ => {
            fail!(UNEXPECTED_STATE, "Invalid state");
          }
        }
      }

      // Update the parser position
      if advanced > 0 {
        self.position += advanced;
        data = &data[advanced..];
        available -= advanced;

        #[cfg(any(debug_assertions, feature = "debug"))]
        if self.debug {
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
      #[cfg(all(not(target_family = "wasm"), any(debug_assertions, feature = "debug")))]
      if self.debug {
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
    if self.debug {
      eprintln!("[milo::debug] loop exit");
    }

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

    #[cfg(all(not(target_family = "wasm"), any(debug_assertions, feature = "debug")))]
    if self.debug {
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

  // RFC 9110 section 6.4.1 - Message completed
  #[inline(always)]
  fn complete(&mut self, offset: usize) {
    callback!(on_message_complete, offset, 0);
    callback!(on_reset, offset, 0);

    self.continue_without_data = false;

    if self.connection == CONNECTION_CLOSE {
      callback!(on_finish);
      move_to!(finish);
    } else {
      move_to!(start);
    }
  }

  #[inline(always)]
  fn find_cr(&self, data: &[u8], available: usize) -> Option<usize> {
    if available == 0 {
      None
    } else {
      self.find_char(data, 0, available - 1, b'\r')
    }
  }

  #[inline(always)]
  fn find_char(&self, buf: &[u8], start: usize, end: usize, needle: u8) -> Option<usize> {
    if start > end || end >= buf.len() {
      return None;
    }

    memchr::memchr(needle, &buf[start..=end]).map(|i| start + i)
  }

  #[inline(always)]
  fn find_char2(&self, buf: &[u8], start: usize, end: usize, needle1: u8, needle2: u8) -> Option<usize> {
    if start > end || end >= buf.len() {
      return None;
    }

    memchr::memchr2(needle1, needle2, &buf[start..=end]).map(|i| start + i)
  }

  #[inline(always)]
  fn ensure_valid_line(&mut self, data: &[u8], cr: usize, available: usize) -> StateResult {
    if cr + 1 == available {
      StateResult::Suspend
    } else if data[cr + 1] != b'\n' {
      self.fail(ERROR_UNEXPECTED_CHARACTER, "Expected CRLF");
      StateResult::Stop
    } else {
      StateResult::Continue
    }
  }

  #[inline(always)]
  fn validate(
    self: &mut Parser,
    data: &[u8],
    range: &[bool; 256],
    start: usize,
    end: usize,
    message: &str,
  ) -> StateResult {
    if start == end {
      self.fail(ERROR_UNEXPECTED_CHARACTER, message);
      return StateResult::Stop;
    }

    let mut i = start;
    while i < end {
      if !range[data[i] as usize] {
        self.fail(ERROR_UNEXPECTED_CHARACTER, message);
        return StateResult::Stop;
      }
      i += 1;
    }

    StateResult::Continue
  }

  #[inline(always)]
  fn validate_allowing_empty(
    self: &mut Parser,
    data: &[u8],
    range: &[bool; 256],
    start: usize,
    end: usize,
    message: &str,
  ) -> StateResult {
    let mut i = start;
    while i < end {
      if !range[data[i] as usize] {
        self.fail(ERROR_UNEXPECTED_CHARACTER, message);
        return StateResult::Stop;
      }
      i += 1;
    }

    StateResult::Continue
  }

  #[inline(always)]
  fn strip_ows(&mut self, data: &[u8], start_ref: &mut usize, end_ref: &mut usize, message: &str) -> StateResult {
    let mut start = *start_ref;
    let mut end = *end_ref;

    while start < end && WS_TABLE[data[start] as usize] {
      start += 1;
    }

    while end > start && WS_TABLE[data[end - 1] as usize] {
      end -= 1;
    }

    if start == end {
      self.fail(ERROR_UNEXPECTED_CHARACTER, message);
      return StateResult::Stop;
    }

    *start_ref = start;
    *end_ref = end;
    StateResult::Continue
  }

  #[inline(always)]
  fn strip_ows_allowing_empty(&self, data: &[u8], start_ref: &mut usize, end_ref: &mut usize) -> StateResult {
    let mut start = *start_ref;
    let mut end = *end_ref;

    while start < end && WS_TABLE[data[start] as usize] {
      start += 1;
    }

    while end > start && WS_TABLE[data[end - 1] as usize] {
      end -= 1;
    }

    *start_ref = start;
    *end_ref = end;
    StateResult::Continue
  }
}
