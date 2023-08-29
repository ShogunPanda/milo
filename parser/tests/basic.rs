#[cfg(test)]
mod test {
  use milo::test_utils::{create_parser, http, length};
  use milo::{Parser, State, PAUSE, REQUEST, RESPONSE};
  use std::ffi::c_char;

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

    parser.values.mode = REQUEST;
    parser.parse(response, length(response));
    assert!(matches!(parser.state, State::ERROR));

    parser.reset();

    parser.values.mode = RESPONSE;
    parser.parse(request, length(request));
    assert!(matches!(parser.state, State::ERROR));
  }

  #[test]
  fn basic_incomplete_string() {
    let mut parser = create_parser();

    let sample1 = http(r#"GET / HTTP/1.1\r"#);
    let sample2 = http(r#"\n"#);
    let sample3 = http(r#"Head"#);
    let sample4 = http(r#"er:"#);
    let sample5 = http(r#"Value"#);
    let sample6 = http(r#"\r\n\r\n"#);

    let consumed1 = parser.parse(sample1, length(sample1));
    assert!(consumed1 == length(sample1) - 1);
    let consumed2 = parser.parse(sample2, length(sample2));
    assert!(consumed2 == length(sample2) + 1);
    let consumed3 = parser.parse(sample3, length(sample3));
    assert!(consumed3 == length(sample3));
    let consumed4 = parser.parse(sample4, length(sample4));
    assert!(consumed4 == length(sample4));
    let consumed5 = parser.parse(sample5, length(sample5));
    assert!(consumed5 == length(sample5));
    let consumed6 = parser.parse(sample6, length(sample6));
    assert!(consumed6 == length(sample6));

    assert!(!matches!(parser.state, State::ERROR));
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

    parser.parse(message, length(message));
    assert!(!matches!(parser.state, State::ERROR));
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

    parser.parse(message, length(message));
    println!("{:?}", parser.state);
    assert!(matches!(parser.state, State::FINISH));
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

    parser.parse(message, length(message));
    assert!(!matches!(parser.state, State::ERROR));
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
        5;ilovew3;somuchlove="arethesepara\"metersfor"\r\n
        hello\r\n
        7;blahblah;blah\r\n
        \s world\r\n
        0\r\n
        Host: example.com\r\n
        Cache-Control: private\r\n\r\n
      "#,
    );

    parser.parse(message, length(message));
    assert!(!matches!(parser.state, State::ERROR));
  }

  #[test]
  fn incomplete_body() {
    let mut parser = create_parser();

    let sample1 = http(r#"POST / HTTP/1.1\r\nContent-Length: 10\r\n\r\n12345"#);
    let sample2 = http(r#"67"#);
    let sample3 = http(r#"890\r\n"#);

    let consumed1 = parser.parse(sample1, length(sample1));
    assert!(consumed1 == length(sample1));
    let consumed2 = parser.parse(sample2, length(sample2));
    assert!(consumed2 == length(sample2));
    let consumed3 = parser.parse(sample3, length(sample3));
    assert!(consumed3 == length(sample3));

    assert!(!matches!(parser.state, State::ERROR));
  }

  #[test]
  fn incomplete_chunk() {
    let mut parser = create_parser();

    let sample1 = http(r#"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\nTrailer: x-foo\r\n\r\na\r\n12345"#);
    let sample2 = http(r#"67"#);
    let sample3 = http(r#"890\r\n0\r\nx-foo: value\r\n\r\n"#);

    let consumed1 = parser.parse(sample1, length(sample1));
    assert!(consumed1 == length(sample1));
    let consumed2 = parser.parse(sample2, length(sample2));
    assert!(consumed2 == length(sample2));
    let consumed3 = parser.parse(sample3, length(sample3));
    assert!(consumed3 == length(sample3));

    assert!(!matches!(parser.state, State::ERROR));
  }

  #[test]
  fn connection_header() {
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

    parser.parse(close_connection, length(close_connection));
    assert!(matches!(parser.state, State::FINISH));

    parser.reset();

    let keep_alive_connection = http(
      r#"
        PUT /url HTTP/1.1\r\n
        Content-Length: 3\r\n
        \r\n
        abc
      "#,
    );

    parser.parse(keep_alive_connection, length(keep_alive_connection));
    assert!(matches!(parser.state, State::START));
  }

  #[test]
  fn pause_and_resume() {
    let mut parser = create_parser();

    let sample1 = http(
      r#"
        PUT /url HTTP/1.1\r\n
        Content-Length: 3\r\n
      "#,
    );
    let sample2 = http(r#"\r\nabc"#); // This will be paused before the body
    let sample3 = http(r#"abc"#);

    parser.callbacks.on_headers_complete =
      Some(|_parser: &mut Parser, _data: *const c_char, _size: usize| -> isize { PAUSE });

    assert!(!parser.paused);

    let consumed1 = parser.parse(sample1, length(sample1));
    assert!(consumed1 == length(sample1));

    assert!(!parser.paused);
    let consumed2 = parser.parse(sample2, length(sample2));
    assert!(consumed2 == length(sample2) - 3);
    assert!(parser.paused);

    let consumed3 = parser.parse(sample3, length(sample3));
    assert!(consumed3 == 0);

    assert!(parser.paused);
    parser.resume();
    assert!(!parser.paused);

    let consumed4 = parser.parse(sample3, length(sample3));
    assert!(consumed4 == length(sample3));
    assert!(!parser.paused);

    assert!(!matches!(parser.state, State::ERROR));
  }

  #[test]
  fn finish() {
    let mut parser = create_parser();

    assert!(matches!(parser.state, State::START));
    parser.finish();
    assert!(matches!(parser.state, State::FINISH));

    parser.reset();

    let close_connection = http(
      r#"
        PUT /url HTTP/1.1\r\n
        Content-Length: 3\r\n
        Connection: close\r\n
        \r\n
        abc
      "#,
    );

    parser.parse(close_connection, length(close_connection));
    assert!(matches!(parser.state, State::FINISH));
    parser.finish();
    assert!(matches!(parser.state, State::FINISH));

    parser.reset();

    let keep_alive_connection = http(
      r#"
        PUT /url HTTP/1.1\r\n
        Content-Length: 3\r\n
        \r\n
        abc
      "#,
    );

    parser.parse(keep_alive_connection, length(keep_alive_connection));
    assert!(matches!(parser.state, State::START));
    parser.finish();
    assert!(matches!(parser.state, State::FINISH));

    parser.reset();

    let incomplete = http(
      r#"
        PUT /url HTTP/1.1\r\n
      "#,
    );

    parser.parse(incomplete, length(incomplete));
    assert!(matches!(parser.state, State::REQUEST_VERSION_COMPLETE));
    parser.finish();
    assert!(matches!(parser.state, State::ERROR));
  }
}
