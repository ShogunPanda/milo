#[cfg(test)]
mod test {
  use milo::test_utils::{create_parser, http, length};
  use milo::State;

  #[test]
  fn upgrade_connect_request() {
    let mut parser = create_parser();

    let message = http(
      r#"
        CONNECT example.com HTTP/1.1\r\n
        Host: example.com\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n\r\n
      "#,
    );

    let consumed1 = parser.parse(message, 0, length(message));
    assert!(consumed1 == 70);
    assert!(matches!(parser.state, State::TUNNEL));

    let consumed2 = parser.parse(message, 0, length(message));
    assert!(consumed2 == 0);
    assert!(matches!(parser.state, State::TUNNEL));
  }

  #[test]
  fn upgrade_connection_upgrade() {
    let mut parser = create_parser();

    let message = http(
      r#"
        GET / HTTP/1.1\r\n
        Host: example.com\r\n
        Connection: upgrade\r\n
        Upgrade: websocket
        Content-Length: 3\r\n
        \r\n
        abc\r\n\r\n
      "#,
    );

    let consumed1 = parser.parse(message, 0, length(message));
    assert!(consumed1 == 95);
    assert!(matches!(parser.state, State::TUNNEL));

    let consumed2 = parser.parse(message, 0, length(message));
    assert!(consumed2 == 0);
    assert!(matches!(parser.state, State::TUNNEL));
  }
}
