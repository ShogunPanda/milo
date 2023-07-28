#[cfg(test)]
mod test {
  use milo::test_utils::{create_parser, http, length};
  use milo::State;

  #[test]
  fn benchmark_seanmonstar_httparse() {
    let message = http(
      r#"
        GET /wp-content/uploads/2010/03/hello-kitty-darth-vader-pink.jpg HTTP/1.1\r\n
        Host: www.kittyhell.com\r\n
        User-Agent: Mozilla/5.0 (Macintosh; U; Intel Mac OS X 10.6; ja-JP-mac; rv:1.9.2.3) Gecko/20100401 Firefox/3.6.3 Pathtraq/0.9\r\n
        Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8\r\n
        Accept-Language: ja,en-us;q=0.7,en;q=0.3\r\n
        Accept-Encoding: gzip,deflate\r\n
        Accept-Charset: Shift_JIS,utf-8;q=0.7,*;q=0.7\r\n
        Keep-Alive: 115\r\n
        Connection: keep-alive\r\n
        Cookie: wp_ozh_wsa_visits=2; wp_ozh_wsa_visit_lasttime=xxxxxxxxxx; __utma=xxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.x; __utmz=xxxxxxxxx.xxxxxxxxxx.x.x.utmccn=(referral)|utmcsr=reader.livedoor.com|utmcct=/reader/|utmcmd=referral\r\n\r\n
      "#,
    );

    let mut parser = create_parser();
    let consumed = parser.parse(message);
    assert!(consumed == length(message));
    assert!(!matches!(parser.state, State::ERROR));
  }

  #[test]
  fn benchmark_nodejs_http_parser() {
    let message = http(
      r#"
        POST /joyent/http-parser HTTP/1.1\r\n
        Host: github.com\r\n
        DNT: 1\r\n
        Accept-Encoding: gzip, deflate, sdch\r\n
        Accept-Language: ru-RU,ru;q=0.8,en-US;q=0.6,en;q=0.4\r\n
        User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_10_1) 
        AppleWebKit/537.36 (KHTML, like Gecko) 
        Chrome/39.0.2171.65 Safari/537.36\r\n
        Accept: text/html,application/xhtml+xml,application/xml;q=0.9,
        image/webp,*/*;q=0.8\r\n
        Referer: https://github.com/joyent/http-parser\r\n
        Connection: keep-alive\r\n
        Transfer-Encoding: chunked\r\n
        Cache-Control: max-age=0\r\n\r\nb\r\nhello world\r\n0\r\n\r\n
      "#,
    );

    let mut parser = create_parser();
    let consumed = parser.parse(message);
    println!("{}, {}", consumed, length(message));
    assert!(consumed == length(message));
    assert!(!matches!(parser.state, State::ERROR));
  }
}
