#[cfg(test)]
mod test {
  use milo::{get_state, STATE_TUNNEL};
  use milo_test_utils::{create_parser, http, parse};

  #[test]
  fn upgrade_connect_request() {
    let parser = create_parser();

    let message1 = http(
      r#"
        CONNECT example.com HTTP/1.1\r\n
        Host: example.com\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n\r\n
      "#,
    );

    let message2 = http(
      r#"
        abc\r\n\r\n
      "#,
    );

    let consumed1 = parse(&parser, &message1);
    assert!(consumed1 == 70);
    assert!(matches!(get_state(&parser), STATE_TUNNEL));

    let consumed2 = parse(&parser, &message2);
    assert!(consumed2 == 0);
    assert!(matches!(get_state(&parser), STATE_TUNNEL));
  }

  #[test]
  fn upgrade_connection_upgrade() {
    let parser = create_parser();

    let message1 = http(
      r#"
        GET / HTTP/1.1\r\n
        Host: example.com\r\n
        Connection: upgrade\r\n
        Upgrade: websocket\r\n
        Content-Length: 3\r\n
        \r\n
        abc\r\n\r\n
      "#,
    );

    let message2 = http(
      r#"
        abc\r\n\r\n
      "#,
    );

    let consumed1 = parse(&parser, &message1);
    assert!(consumed1 == 97);
    assert!(matches!(get_state(&parser), STATE_TUNNEL));

    let consumed2 = parse(&parser, &message2);
    assert!(consumed2 == 0);
    assert!(matches!(get_state(&parser), STATE_TUNNEL));
  }
}
