#![no_main]
#[macro_use]
extern crate libfuzzer_sys;

use milo_fuzz::MAX_FEED;
use milo_parser::{CALLBACK_ACTIVE_ALL, Parser};

// Layer 1: raw byte-level fuzzing of the parser entry point, with every config
// toggle that changes control flow exercised. This is the crash/UB target.
fuzz_target!(|data: &[u8]| {
  let limit = data.len().min(MAX_FEED);

  // (a) autodetect mode, all events enabled.
  {
    let mut p = Parser::new();
    p.active_events = CALLBACK_ACTIVE_ALL;
    let _ = p.parse(data.as_ptr(), limit);
  }

  // (b) forced request and response modes, with unconsumed management enabled so
  // the retained-buffer allocation and cross-call aggregation paths are hit.
  for is_request in [false, true] {
    let mut p = Parser::new();
    p.autodetect = false;
    p.is_request = is_request;
    p.active_events = CALLBACK_ACTIVE_ALL;
    p.manage_unconsumed = true;

    if data.len() > 1 {
      let (a, b) = data.split_at(data.len() / 2);
      // The second call prepends any bytes retained by the first.
      let _ = p.parse(a.as_ptr(), a.len());
      let _ = p.parse(b.as_ptr(), b.len());
    } else {
      let _ = p.parse(data.as_ptr(), limit);
    }
  }

  // (c) body-skipping / suspend-after-headers paths.
  {
    let mut p = Parser::new();
    p.active_events = CALLBACK_ACTIVE_ALL;
    p.skip_body = true;
    p.suspend_after_headers = true;
    let _ = p.parse(data.as_ptr(), limit);
  }

  // (d) body payload cap.
  {
    let mut p = Parser::new();
    p.active_events = CALLBACK_ACTIVE_ALL;
    p.max_body_payload = 1;
    let _ = p.parse(data.as_ptr(), limit);
  }
});
