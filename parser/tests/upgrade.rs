mod helpers;

use helpers::{create_parser, http, parse};
use milo::STATE_TUNNEL;

#[test]
fn upgrade_connect_request() {
  let mut parser = create_parser();

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

  let consumed1 = parse(&mut parser, &message1);
  assert_eq!(consumed1, 70);
  assert_eq!(parser.state, STATE_TUNNEL);

  let consumed2 = parse(&mut parser, &message2);
  assert_eq!(consumed2, 0);
  assert_eq!(parser.state, STATE_TUNNEL);
}

#[test]
fn upgrade_connection_upgrade() {
  let mut parser = create_parser();

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

  let consumed1 = parse(&mut parser, &message1);
  assert_eq!(consumed1, message1.len() - message2.len());
  assert_eq!(parser.state, STATE_TUNNEL);

  let consumed2 = parse(&mut parser, &message2);
  assert_eq!(consumed2, 0);
  assert_eq!(parser.state, STATE_TUNNEL);
}

#[test]
fn upgrade_http_101() {
  let mut parser = create_parser();

  let message1 = http(
    r#"
        HTTP/1.1 101 Switching Protocols\r\n
        hello: world\r\n
        connection: upgrade\r\n
        upgrade: websocket\r\n
        \r\n
        Body
      "#,
  );

  let message2 = http(
    r#"
        abc\r\n\r\n
      "#,
  );

  let consumed1 = parse(&mut parser, &message1);
  assert_eq!(consumed1, message1.len() - 4);
  assert_eq!(parser.state, STATE_TUNNEL);

  let consumed2 = parse(&mut parser, &message2);
  assert_eq!(consumed2, 0);
  assert_eq!(parser.state, STATE_TUNNEL);
}
