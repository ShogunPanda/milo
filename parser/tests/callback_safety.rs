use core::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

use milo_parser::{
  CALLBACK_ACTIVE_ON_MESSAGE_START, CALLBACK_ACTIVE_ON_REQUEST, ERROR_NONE, ERROR_UNEXPECTED_CHARACTER, Parser,
  STATE_START, milo_create, milo_destroy, milo_parse,
};

#[derive(Default)]
struct ReentryContext {
  nested_consumed: usize,
  mutation_rejected: bool,
  message_start_called: bool,
}

fn noop(_parser: &mut Parser, _at: usize, _len: usize) {}

fn attempt_reentry(parser: &mut Parser, _at: usize, _len: usize) {
  let state = parser.state;
  let parsed = parser.parsed;
  let error_code = parser.error_code;
  let nested = b"GET /nested HTTP/1.1\r\n\r\n";

  let nested_consumed = parser.parse(nested.as_ptr(), nested.len());
  parser.pause();
  parser.reset(false);
  parser.clear();
  parser.complete();
  parser.finish();
  parser.fail(ERROR_UNEXPECTED_CHARACTER, "callback mutation");

  let context = unsafe { &mut *(parser.context as *mut ReentryContext) };
  context.nested_consumed = nested_consumed;
  context.mutation_rejected =
    parser.state == state && parser.parsed == parsed && parser.error_code == error_code && !parser.paused;

  // The current callback batch uses immutable callback and activation snapshots.
  parser.active_callbacks = 0;
  parser.callbacks.on_message_start = noop;
}

fn record_message_start(parser: &mut Parser, _at: usize, _len: usize) {
  let context = unsafe { &mut *(parser.context as *mut ReentryContext) };
  context.message_start_called = true;
}

#[test]
fn callback_reentry_and_mutation_are_rejected() {
  let mut parser = Parser::new();
  parser.autodetect = false;
  parser.is_request = true;
  let context = Box::new(ReentryContext::default());
  parser.context = Box::into_raw(context) as *mut c_void;
  parser.active_callbacks = CALLBACK_ACTIVE_ON_REQUEST | CALLBACK_ACTIVE_ON_MESSAGE_START;
  parser.callbacks.on_request = attempt_reentry;
  parser.callbacks.on_message_start = record_message_start;

  let message = b"GET / HTTP/1.1\r\n\r\n";
  assert_eq!(parser.parse(message.as_ptr(), message.len()), message.len());
  assert_eq!(parser.state, STATE_START);
  assert_eq!(parser.error_code, ERROR_NONE);

  let context = unsafe { Box::from_raw(parser.context as *mut ReentryContext) };
  assert_eq!(context.nested_consumed, 0);
  assert!(context.mutation_rejected);
  assert!(context.message_start_called);
  parser.context = core::ptr::null_mut();
}

static CALLBACK_AFTER_DESTROY: AtomicBool = AtomicBool::new(false);

fn destroy_during_callback(parser: &mut Parser, _at: usize, _len: usize) { milo_destroy(parser as *mut Parser); }

fn callback_after_destroy(_parser: &mut Parser, _at: usize, _len: usize) {
  CALLBACK_AFTER_DESTROY.store(true, Ordering::SeqCst);
}

#[test]
fn callback_destruction_is_deferred_and_stops_replay() {
  CALLBACK_AFTER_DESTROY.store(false, Ordering::SeqCst);
  let parser = milo_create();
  unsafe {
    (*parser).autodetect = false;
    (*parser).is_request = true;
    (*parser).active_callbacks = CALLBACK_ACTIVE_ON_REQUEST | CALLBACK_ACTIVE_ON_MESSAGE_START;
    (*parser).callbacks.on_request = destroy_during_callback;
    (*parser).callbacks.on_message_start = callback_after_destroy;
  }

  let message = b"GET / HTTP/1.1\r\n\r\n";
  assert_eq!(milo_parse(parser, message.as_ptr(), message.len()), message.len());
  assert!(!CALLBACK_AFTER_DESTROY.load(Ordering::SeqCst));
}
