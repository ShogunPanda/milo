#![feature(vec_into_raw_parts)]

use std::{env, ffi::c_void};

use milo::{create, Parser};
use regex::Regex;

pub mod context;
mod output;

#[path = "./callbacks.rs"]
pub mod callbacks;

pub fn create_parser() -> Parser {
  let parser = create(None);
  let context = Box::new(context::Context::new());
  parser.owner.set(Box::into_raw(context) as *mut c_void);

  if env::var_os("DEBUG_TESTS").unwrap_or("false".into()) == "true" {
    parser.callbacks.before_state_change.set(callbacks::before_state_change);
    parser.callbacks.after_state_change.set(callbacks::after_state_change);
  }

  parser.callbacks.on_error.set(callbacks::on_error);
  parser.callbacks.on_finish.set(callbacks::on_finish);
  parser.callbacks.on_request.set(callbacks::on_request);
  parser.callbacks.on_response.set(callbacks::on_response);
  parser.callbacks.on_message_start.set(callbacks::on_message_start);
  parser.callbacks.on_message_complete.set(callbacks::on_message_complete);
  parser.callbacks.on_method.set(callbacks::on_method);
  parser.callbacks.on_url.set(callbacks::on_url);
  parser.callbacks.on_protocol.set(callbacks::on_protocol);
  parser.callbacks.on_version.set(callbacks::on_version);
  parser.callbacks.on_status.set(callbacks::on_status);
  parser.callbacks.on_reason.set(callbacks::on_reason);
  parser.callbacks.on_header_name.set(callbacks::on_header_name);
  parser.callbacks.on_header_value.set(callbacks::on_header_value);
  parser.callbacks.on_headers.set(callbacks::on_headers);
  parser.callbacks.on_upgrade.set(callbacks::on_upgrade);
  parser.callbacks.on_chunk_length.set(callbacks::on_chunk_length);
  parser
    .callbacks
    .on_chunk_extension_name
    .set(callbacks::on_chunk_extension_name);
  parser
    .callbacks
    .on_chunk_extension_value
    .set(callbacks::on_chunk_extension_value);
  parser.callbacks.on_chunk.set(callbacks::on_chunk);
  parser.callbacks.on_body.set(callbacks::on_body);
  parser.callbacks.on_data.set(callbacks::on_data);
  parser.callbacks.on_trailer_name.set(callbacks::on_trailer_name);
  parser.callbacks.on_trailer_value.set(callbacks::on_trailer_value);
  parser.callbacks.on_trailers.set(callbacks::on_trailers);

  parser
}

pub fn http(input: &str) -> String {
  let trailing_ws = Regex::new(r"(?m)^\s+").unwrap();

  trailing_ws
    .replace_all(input.trim(), "")
    .replace('\n', "")
    .replace("\\r", "\r")
    .replace("\\n", "\n")
    .replace("\\s", " ")
}

pub fn output(input: &str) -> String { String::from(input.trim()) + "\n" }

pub fn parse(parser: &Parser, content: &str) -> usize {
  let mut context = unsafe { Box::from_raw(parser.owner.get() as *mut context::Context) };
  context.input = String::from(content);
  Box::into_raw(context);

  milo::parse(parser, content.as_ptr(), content.len())
}
