#[cfg(test)]
mod test {
  #[allow(unused_imports)]
  use std::ffi::c_uchar;

  use milo::STATE_ERROR;
  use milo_test_utils::{context, create_parser, http, parse};

  #[test]
  fn undici() {
    let message = http(
      r#"
        HTTP/1.1 200 OK\r\n
        Connection: keep-alive\r\n
        Content-Length: 65535\r\n
        Date: Sun, 05 Nov 2023 14:26:18 GMT\r\n
        Keep-Alive: timeout=600\r\n\r\n
        @
      "#,
    )
    .replace('@', &format!("{:-<65535}", "-"));

    let mut parser = create_parser();
    parse(&mut parser, &message);
    assert!(!matches!(parser.state, STATE_ERROR));
    assert!((parser.parsed as usize) == message.len());
  }

  #[test]
  fn undici_multiple() {
    let message1 = http(
      r#"
        HTTP/1.1 200 OK\r\n
        Date: Mon, 08 Apr 2024 13:20:53 GMT\r\n
        Connection: keep-alive\r\n
        Keep-Alive: timeout=5\r\n
        Transfer-Encoding: chunked\r\n
        \r\n
        3e80\r\n
        @\r\n
        3e80\r\n
        @\r\n
        0\r\n\r\n
      "#,
    )
    .replace('@', &format!("{:-<16000}", "-"));

    let mut parser = create_parser();
    parse(&mut parser, &message1);
    assert!(!matches!(parser.state, STATE_ERROR));
  }

  #[test]
  fn undici_multibyte() {
    let message = http(
      r#"
      HTTP/1.1 200 OK\r\n
      Date: Tue, 09 Apr 2024 10:39:04 GMT\r\n
      Connection: keep-alive\r\n
      Keep-Alive: timeout=5\r\n
      Content-Length: 300010\r\n
      \r\n
      {"asd":"@"}
      "#,
    )
    .replace('@', &format!("{:あ<100000}", "あ"));

    let mut parser = create_parser();

    let mut length = message.len();
    let mut offset = 0;
    let mut step = 65536;

    while length > 0 {
      let mut context = unsafe { Box::from_raw(parser.context as *mut context::Context) };

      context.input = unsafe { String::from_utf8_unchecked(message.as_bytes()[offset..(offset + step)].to_vec()) };
      Box::into_raw(context);

      parser.parse(unsafe { message.as_ptr().add(offset) }, step);
      assert!(!matches!(parser.state, STATE_ERROR));

      length -= step;
      offset += step;
      step = if length > 65536 { 65536 } else { length };
    }
  }
}
