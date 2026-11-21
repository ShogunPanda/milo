use core::ptr;

use milo_parser::*;

const REQUEST: &[u8] = b"GET / HTTP/1.1\r\nHost: example.test\r\n\r\n";

extern "C" fn on_start(parser: &mut Parser, _: usize, _: usize) {
  assert_eq!(parser.parse(REQUEST), 0);
  let raw = ptr::from_mut(parser);
  // SAFETY: The raw handle comes from this callback's live parser borrow.
  unsafe {
    assert_eq!(milo_parse(raw, REQUEST.as_ptr(), REQUEST.len()), 0);
    milo_reset(raw, false);
    milo_finish(raw);
    milo_destroy(raw);
    milo_set_active_events(raw, EVENT_ACTIVE_ALL);
    milo_set_max_body_payload(raw, 1);
    milo_set_suspend_after_headers(raw, true);
  }
  assert_eq!(parser.active_events, 0);
  assert_eq!(parser.max_body_payload, 0);
  assert!(!parser.suspend_after_headers);
  assert_eq!(parser.parsed, REQUEST.len() as u64);
  assert_eq!(parser.state, STATE_START);
  parser.pause();
  assert_eq!(Parser::new().parse(REQUEST), REQUEST.len());
}

extern "C" fn on_headers(parser: &mut Parser, _: usize, _: usize) {
  assert!(parser.paused);
  parser.resume();
}

extern "C" fn replace_parser(parser: &mut Parser, _: usize, _: usize) { *parser = Parser::new(); }

extern "C" fn unexpected_headers(_: &mut Parser, _: usize, _: usize) {
  panic!("old event batch was replayed");
}

#[test]
fn callback_controls_do_not_change_the_batch() {
  let raw = milo_create();
  // SAFETY: The parser is FFI-owned and the fixture outlives the call.
  unsafe {
    (*raw).active_callbacks = CALLBACK_ACTIVE_ON_MESSAGE_START | CALLBACK_ACTIVE_ON_HEADERS;
    (*raw).callbacks.on_message_start = on_start;
    (*raw).callbacks.on_headers = on_headers;
    assert_eq!(milo_parse(raw, REQUEST.as_ptr(), REQUEST.len()), REQUEST.len());
    assert!(!(*raw).paused);
    milo_reset(raw, false);
    assert_eq!((*raw).parsed, 0);
    milo_finish(raw);
    assert_eq!((*raw).state, STATE_FINISH);
    milo_destroy(raw);
  }
}

#[test]
fn replacing_the_parser_does_not_read_its_old_batch() {
  let mut parser = Parser::new();
  parser.active_callbacks = CALLBACK_ACTIVE_ON_MESSAGE_START | CALLBACK_ACTIVE_ON_HEADERS;
  parser.callbacks.on_message_start = replace_parser;
  parser.callbacks.on_headers = unexpected_headers;
  assert_eq!(parser.parse(REQUEST), REQUEST.len());
  assert_eq!(parser.parse(REQUEST), REQUEST.len());
}
