#[cfg(test)]
mod tests {
  extern crate test;

  use milo::Parser;
  use std::ffi::CString;
  use test::Bencher;

  fn raw(input: &str) -> *mut i8 {
    CString::new(input.replace("\n", "\r\n")).unwrap().into_raw()
  }

  #[bench]
  fn content_length(b: &mut Bencher) {
    let payload = raw("
PUT /url HTTP/1.1
Content-Length: 003

abc
");

    let mut parser = Parser::new();

    b.iter(|| {
      parser.reset();
      parser.parse(payload)
    });
  }

  #[bench]
  fn chunked_encoding(b: &mut Bencher) {
    let payload = raw(
      "
POST /chunked_w_unicorns_after_length HTTP/1.1
Transfer-Encoding: chunked

5;ilovew3;somuchlove=aretheseparametersfor
hello
7;blahblah;blah
    world
0
"
    );

    let mut parser = Parser::new();

    b.iter(|| {
      parser.reset();
      parser.parse(payload)
    });
  }
}
