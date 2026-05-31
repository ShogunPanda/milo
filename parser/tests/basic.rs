mod helpers;

#[allow(unused_imports)]
use std::ffi::c_uchar;

use milo_parser::{CALLBACK_ACTIVE_ON_HEADERS, Parser, STATE_ERROR, STATE_FINISH, STATE_HEADER, STATE_START};

use crate::helpers::{context, create_parser, http, parse};

#[test]
fn basic_disable_autodetect() {
  let mut parser = create_parser();

  let request = http(
    r#"
        PUT /url HTTP/1.1\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n\r\n
      "#,
  );

  let response = http(
    r#"
        HTTP/1.1 200 OK\r\n
        Header1: Value1\r\n
        Header2: Value2\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n\r\n
      "#,
  );

  parser.autodetect = false;
  parser.is_request = true;
  parse(&mut parser, &response);
  assert_eq!(parser.state, STATE_ERROR);

  parser.reset(false);

  parser.autodetect = false;
  parser.is_request = false;
  parse(&mut parser, &request);
  assert_eq!(parser.state, STATE_ERROR);
}

#[test]
fn basic_incomplete_string_1() {
  let mut parser = create_parser();

  let sample1 = http(r#"GET / HTTP/1.1\r"#);
  let sample2 = http(r#"1.1\r\n"#);
  let sample3 = http(r#"Head"#);
  let sample4 = http(r#"Header:"#);
  let sample5 = http(r#"Value"#);
  let sample6 = http(r#"Value\r\n\r\n"#);

  let consumed1 = parse(&mut parser, &sample1);
  assert_eq!(consumed1, 0);
  let consumed2 = parse(&mut parser, &sample2);
  assert_eq!(consumed2, 0);
  let consumed3 = parse(&mut parser, &sample3);
  assert_eq!(consumed3, 0);
  let consumed4 = parse(&mut parser, &sample4);
  assert_eq!(consumed4, 0);
  let consumed5 = parse(&mut parser, &sample5);
  assert_eq!(consumed5, 0);
  let consumed6 = parse(&mut parser, &sample6);
  assert_eq!(consumed6, 0);

  assert_eq!(parser.state, STATE_ERROR);
}

#[test]
fn basic_incomplete_string_2() {
  let mut parser = create_parser();

  parser.autodetect = false;
  parser.is_request = true;
  let sample1 = http(r#"GE"#);
  let sample2 = http(r#"GET / HTTP/1.1\r\nHost: foo\r\n\r\n"#);

  let consumed1 = parse(&mut parser, &sample1);
  assert_eq!(consumed1, 0);

  let consumed2 = parse(&mut parser, &sample2);
  assert_eq!(consumed2, sample2.len());

  assert_ne!(parser.state, STATE_ERROR);
}

#[test]
fn basic_incomplete_string_automanaged_1() {
  let mut parser = create_parser();
  parser.manage_unconsumed = true;

  let message = http(r"GET / HTTP/1.1\r\nHeader: Value\r\n\r\n");
  let sample1 = &message[0..15].to_string(); // GET / HTTP/1.1\r
  let sample2 = &message[15..16].to_string(); // \n
  let sample3 = &message[16..20].to_string(); // Head
  let sample4 = &message[20..24].to_string(); // er:
  let sample5 = &message[24..29].to_string(); // Value
  let sample6 = &message[29..].to_string(); // \r\n\r\n

  let mut context = unsafe { Box::from_raw(parser.context as *mut context::Context) };
  context.input = message.clone();
  let _ = Box::into_raw(context);

  let consumed1 = parser.parse(sample1.as_ptr(), sample1.len());
  assert_eq!(consumed1, 0);
  let consumed2 = parser.parse(sample2.as_ptr(), sample2.len());
  assert_eq!(consumed2, 16);
  let consumed3 = parser.parse(sample3.as_ptr(), sample3.len());
  assert_eq!(consumed3, 0);
  let consumed4 = parser.parse(sample4.as_ptr(), sample4.len());
  assert_eq!(consumed4, 0);
  let consumed5 = parser.parse(sample5.as_ptr(), sample5.len());
  assert_eq!(consumed5, 0);
  let consumed6 = parser.parse(sample6.as_ptr(), sample6.len());
  assert_eq!(consumed6, sample3.len() + sample4.len() + sample5.len() + sample6.len());

  assert_ne!(parser.state, STATE_ERROR);
  assert_eq!(parser.parsed, message.len() as u64);

  // Verify the field is not reset
  parser.reset(true);

  let consumed1 = parser.parse(sample1.as_ptr(), sample1.len());
  assert_eq!(consumed1, 0);
  let consumed2 = parser.parse(sample2.as_ptr(), sample2.len());
  assert_eq!(consumed2, 16);
  let consumed3 = parser.parse(sample3.as_ptr(), sample3.len());
  assert_eq!(consumed3, 0);
  let consumed4 = parser.parse(sample4.as_ptr(), sample4.len());
  assert_eq!(consumed4, 0);
  let consumed5 = parser.parse(sample5.as_ptr(), sample5.len());
  assert_eq!(consumed5, 0);
  let consumed6 = parser.parse(sample6.as_ptr(), sample6.len());
  assert_eq!(consumed6, sample3.len() + sample4.len() + sample5.len() + sample6.len());

  assert_ne!(parser.state, STATE_ERROR);
  assert_eq!(parser.parsed, (message.len() * 2) as u64);
}

#[test]
fn basic_incomplete_string_automanaged_2() {
  let mut parser = create_parser();
  parser.manage_unconsumed = true;
  parser.autodetect = false;
  parser.is_request = true;

  let message = http(r#"GET / HTTP/1.1\r\nHost: foo\r\n\r\n"#);

  let mut context = unsafe { Box::from_raw(parser.context as *mut context::Context) };
  context.input = message.clone();
  let _ = Box::into_raw(context);

  let sample1 = &message[0..2].to_string(); // GE
  let sample2 = &message[2..].to_string(); // T / HTTP/1.1\r\nHost: foo\r\n\r\n

  parser.parse(sample1.as_ptr(), sample1.len());
  parser.parse(sample2.as_ptr(), sample2.len());

  assert_ne!(parser.state, STATE_ERROR);
  assert_eq!(parser.parsed, message.len() as u64);
}

#[test]
fn basic_sample_multiple_requests() {
  let mut parser = create_parser();

  let message = http(
    r#"
        POST /chunked_w_unicorns_after_length HTTP/1.1\r\n
        Transfer-Encoding: chunked\r\n
        \r\n
        5;ilovew3;somuchlove=aretheseparametersfor\r\n
        hello\r\n
        7;blahblah;blah\r\n
        \s world\r\n
        0\r\n\r\n
        \r\n
        POST / HTTP/1.1\r\n
        Host: www.example.com\r\n
        Content-Type: application/x-www-form-urlencoded\r\n
        Content-Length: 4\r\n
        \r\n
        q=42\r\n
        \r\n
        GET / HTTP/1.1\r\n\r\n
      "#,
  );

  parse(&mut parser, &message);
  assert_ne!(parser.state, STATE_ERROR);
}

#[test]
fn basic_connection_close() {
  let mut parser = create_parser();

  let message = http(
    r#"
        POST /chunked_w_unicorns_after_length HTTP/1.1\r\n
        Connection: close\r\n
        Transfer-Encoding: chunked\r\n
        \r\n
        5;ilovew3;somuchlove=aretheseparametersfor\r\n
        hello\r\n
        7;blahblah;blah\r\n
        \s world\r\n
        0\r\n\r\n
      "#,
  );

  parse(&mut parser, &message);

  assert_eq!(parser.state, STATE_FINISH);
}

#[test]
fn basic_sample_multiple_responses() {
  let mut parser = create_parser();

  let message = http(
    r#"
        HTTP/1.1 200 OK\r\n
        Header1: Value1\r\n
        Header2: Value2\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n\r\n
        HTTP/1.1 200 OK\r\n
        Header1: Value1\r\n
        Header2: Value2\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n
        HTTP/1.1 200 OK\r\n
        Header1: Value1\r\n
        Header2: Value2\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n\r\n
      "#,
  );

  parse(&mut parser, &message);
  assert_ne!(parser.state, STATE_ERROR);
}

#[test]
fn basic_trailers() {
  let mut parser = create_parser();

  let message = http(
    r#"
        POST /chunked_w_unicorns_after_length HTTP/1.1\r\n
        Transfer-Encoding: chunked\r\n
        Trailer: host,cache-control\r\n
        \r\n
        5;ilovew3;somuchlove="arethesepara\"metersfor";another="1111\"2222\"3333"\r\n
        hello\r\n
        7;blahblah;blah;somuchlove="arethesepara"\r\n
        \s world\r\n
        0\r\n
        Host: example.com\r\n
        Cache-Control: private\r\n\r\n
      "#,
  );

  parse(&mut parser, &message);
  assert_ne!(parser.state, STATE_ERROR);
}

#[test]
fn basic_incomplete_body() {
  let mut parser = create_parser();

  let sample1 = http(r#"POST / HTTP/1.1\r\nContent-Length:10\r\n\r\n12345"#);
  let sample2 = http(r#"67"#);
  let sample3 = http(r#"890\r\n"#);

  let consumed1 = parse(&mut parser, &sample1);
  assert_eq!(consumed1, sample1.len());
  let consumed2 = parse(&mut parser, &sample2);
  assert_eq!(consumed2, sample2.len());
  let consumed3 = parse(&mut parser, &sample3);
  assert_eq!(consumed3, sample3.len());

  assert_ne!(parser.state, STATE_ERROR);
}

#[test]
fn basic_incomplete_chunk() {
  let mut parser = create_parser();

  let sample1 = http(r#"POST / HTTP/1.1\r\nTransfer-Encoding:chunked\r\nTrailer: x-foo\r\n\r\na\r\n12345"#);
  let sample2 = http(r#"67"#);
  let sample3 = http(r#"890\r\n0\r\nx-foo:value\r\n\r\n"#);

  let consumed1 = parse(&mut parser, &sample1);
  assert_eq!(consumed1, sample1.len());
  let consumed2 = parse(&mut parser, &sample2);
  assert_eq!(consumed2, sample2.len());
  let consumed3 = parse(&mut parser, &sample3);
  assert_eq!(consumed3, sample3.len());

  assert_ne!(parser.state, STATE_ERROR);
}

#[test]
fn basic_connection_header() {
  let mut parser = create_parser();

  let close_connection = http(
    r#"
        PUT /url HTTP/1.1\r\n
        Content-Length: 3\r\n
        Connection: close\r\n
        \r\n
        abc
      "#,
  );

  parse(&mut parser, &close_connection);
  assert_eq!(parser.state, STATE_FINISH);

  parser.reset(false);

  let keep_alive_connection = http(
    r#"
        PUT /url HTTP/1.1\r\n
        Content-Length: 3\r\n
        \r\n
        abc
      "#,
  );

  parse(&mut parser, &keep_alive_connection);

  assert_eq!(parser.state, STATE_START);
}

#[test]
fn basic_pause_and_resume() {
  let mut parser = create_parser();

  let sample1 = http(
    r#"
        PUT /url HTTP/1.1\r\n
        Content-Length: 3\r\n
      "#,
  );
  let sample2 = http(r#"\r\nabc"#); // This will be paused before the body
  let sample3 = http(r#"abc"#);

  parser.callbacks.on_headers = |p: &mut Parser, _at: usize, _size: usize| {
    p.pause();
  };
  parser.active_callbacks |= CALLBACK_ACTIVE_ON_HEADERS;

  assert_eq!(parser.paused, false);

  let consumed1 = parse(&mut parser, &sample1);
  assert_eq!(consumed1, sample1.len());

  assert_eq!(parser.paused, false);
  let consumed2 = parse(&mut parser, &sample2);
  assert_eq!(consumed2, sample2.len() - 3);
  assert_eq!(parser.paused, true);

  let consumed3 = parse(&mut parser, &sample3);
  assert_eq!(consumed3, 0);

  assert_eq!(parser.paused, true);
  parser.resume();
  assert_eq!(parser.paused, false);

  let consumed4 = parse(&mut parser, &sample3);
  assert_eq!(consumed4, sample3.len());
  assert_eq!(parser.paused, false);

  assert_ne!(parser.state, STATE_ERROR);
}

#[test]
fn basic_restart() {
  let mut parser = create_parser();
  parser.autodetect = false;
  parser.is_request = false;

  let response = http(
    r#"
        HTTP/1.1 200 OK\r\n
        Header1: Value1\r\n
        Header2: Value2\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n\r\n
        HTTP/1.1 200 OK\r\n
        Header1: Value1\r\n
        Header2: Value2\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n
        HTTP/1.1 200 OK\r\n
        Header1: Value1\r\n
        Header2: Value2\r\n
        Content-Length: 3\r\n
        \r\n
        abc
      "#,
  );

  let request = http(
    r#"
        PUT /url HTTP/1.1\r\n
        Content-Length: 3\r\n
        Connection: keep-alive\r\n
        \r\n
        abc
      "#,
  );

  parse(&mut parser, &response);
  assert_eq!(parser.state, STATE_START);

  parser.autodetect = false;
  parser.is_request = true;
  parser.reset(false);

  parse(&mut parser, &request);
  assert_eq!(parser.state, STATE_START);
}

#[test]
fn basic_finish_logic() {
  let mut parser = create_parser();

  assert_eq!(parser.state, STATE_START);
  parser.finish();
  assert_eq!(parser.state, STATE_FINISH);

  parser.reset(false);

  let close_connection = http(
    r#"
        PUT /url HTTP/1.1\r\n
        Content-Length: 3\r\n
        Connection: close\r\n
        \r\n
        abc
      "#,
  );

  parse(&mut parser, &close_connection);
  assert_eq!(parser.state, STATE_FINISH);
  parser.finish();
  assert_eq!(parser.state, STATE_FINISH);

  parser.reset(false);

  let keep_alive_connection = http(
    r#"
        PUT /url HTTP/1.1\r\n
        Content-Length: 3\r\n
        \r\n
        abc
      "#,
  );

  parse(&mut parser, &keep_alive_connection);
  assert_eq!(parser.state, STATE_START);
  parser.finish();
  assert_eq!(parser.state, STATE_FINISH);

  parser.reset(false);

  let incomplete = http(
    r#"
        PUT /url HTTP/1.1\r\n
      "#,
  );

  parse(&mut parser, &incomplete);

  assert_eq!(parser.state, STATE_HEADER);
  parser.finish();
  assert_eq!(parser.state, STATE_ERROR);
}

#[test]
fn basic_empty_fields() {
  let mut parser = create_parser();

  let message = http(
    r#"
        POST / HTTP/1.1\r\n
        Transfer-Encoding: chunked\r\n
        Content-Type: \r\n
        Trailer: host\r\n
        \r\n
        0\r\n
        Host:\r\n\r\n
      "#,
  );

  parse(&mut parser, &message);
  assert_ne!(parser.state, STATE_ERROR);
}

#[test]
fn basic_space_after_header_name() {
  let mut parser = create_parser();

  let message = http(
    r#"
        PUT /url HTTP/1.1\r\n
        Content-Length : 3\r\n
        \r\n
        abc\r\n\r\n
      "#,
  );

  parse(&mut parser, &message);
  assert_eq!(parser.state, STATE_ERROR);
}

#[test]
fn basic_response_204_has_no_body() {
  let mut parser = create_parser();
  parser.autodetect = false;
  parser.is_request = false;

  let message = http(
    r#"
        HTTP/1.1 204 No content\r\n\r\n
        HTTP/1.1 200 OK\r\n
        Content-Length: 2\r\n
        \r\n
        ok
      "#,
  );

  parse(&mut parser, &message);
  assert_eq!(parser.state, STATE_START);
}
