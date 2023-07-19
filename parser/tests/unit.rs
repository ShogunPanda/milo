#[path = "../src/test_utils.rs"]
mod test_utils;

#[cfg(test)]
mod test {
  use crate::test_utils::{ create_parser, http };
  use milo::{ State, REQUEST, RESPONSE };
  use std::ffi::CString;

  #[test]
  fn disable_autodetect() {
    let mut parser = create_parser();

    let request = http("
PUT /url HTTP/1.1
Content-Length: 3

abc
  ");

    let response = http("
HTTP/1.1 200 OK
Header1: Value1
Header2: Value2
Content-Length: 3

abc
  ");

    parser.values.mode = REQUEST;
    parser.parse(CString::new(response).unwrap().into_raw());
    assert!(matches!(parser.state, State::ERROR));

    parser.reset();

    parser.values.mode = RESPONSE;
    parser.parse(CString::new(request).unwrap().into_raw());
    assert!(matches!(parser.state, State::ERROR));
  }

  #[test]
  fn incomplete_string() {
    let mut parser = create_parser();

    let sample1 = "GET / HTTP/1.1\r";
    let sample2 = "\n\r\n";

    let consumed1 = parser.parse(CString::new(sample1).unwrap().into_raw());
    assert!(consumed1 == sample1.len() - 1);

    let consumed2 = parser.parse(CString::new(format!("\r{}", sample2)).unwrap().into_raw());
    assert!(consumed2 == sample2.len() + 1);

    assert!(!matches!(parser.state, State::ERROR));
  }

  #[test]
  fn sample_multiple_requests() {
    let mut parser = create_parser();

    let sample = http(
      "
POST /chunked_w_unicorns_after_length HTTP/1.1
Transfer-Encoding: chunked

5;ilovew3;somuchlove=aretheseparametersfor
hello
7;blahblah;blah
  world
0

POST / HTTP/1.1
Host: www.example.com
Content-Type: application/x-www-form-urlencoded
Content-Length: 4

q=42

GET / HTTP/1.1

  "
    );

    parser.parse(CString::new(sample).unwrap().into_raw());
    assert!(!matches!(parser.state, State::ERROR));
  }

  #[test]
  fn sample_multiple_responses() {
    let mut parser = create_parser();

    let sample = http(
      "
HTTP/1.1 200 OK
Header1: Value1
Header2: Value2
Content-Length: 3

abc
HTTP/1.1 200 OK
Header1: Value1
Header2: Value2
Content-Length: 3

abc
HTTP/1.1 200 OK
Header1: Value1
Header2: Value2
Content-Length: 3

abc
  "
    );

    parser.parse(CString::new(sample).unwrap().into_raw());
    assert!(!matches!(parser.state, State::ERROR));
  }

  #[test]
  fn trailers() {
    let mut parser = create_parser();

    let sample = http(
      "
POST /chunked_w_unicorns_after_length HTTP/1.1
Transfer-Encoding: chunked
Trailer: host,cache-control

5;ilovew3;somuchlove=\"arethesepara\\\"metersfor\"
hello
7;blahblah;blah
  world
0
Host: example.com
Cache-Control: private
  "
    );

    parser.parse(CString::new(sample).unwrap().into_raw());
    assert!(!matches!(parser.state, State::ERROR));
  }
}
