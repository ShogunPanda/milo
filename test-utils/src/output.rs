use core::{ffi::c_uchar, slice, str};

use crate::{context, Parser};

pub fn extract_payload(parser: &Parser, from: usize, size: usize) -> (*const c_uchar, impl Fn()) {
  let context = unsafe { Box::from_raw(parser.context as *mut context::Context) };
  let (ptr, len, cap) = Vec::into_raw_parts(context.input.as_bytes().into());
  Box::into_raw(context);

  (
    if size > 0 {
      unsafe { ptr.add(from) }
    } else {
      std::ptr::null()
    },
    move || {
      unsafe { Vec::from_raw_parts(ptr, len, cap) };
    },
  )
}

#[allow(dead_code)]
pub fn format_event(name: &str) -> String { format!("\"{}\"", name).to_string() }

#[allow(dead_code)]
pub fn append_output(parser: &Parser, message: String, from: usize, size: usize) {
  let (data, cleanup) = extract_payload(parser, from, size);

  let formatted = format!(
    "{{ {}, \"data\": {} }}\n",
    message,
    if !data.is_null() {
      format!("\"{}\"", unsafe {
        str::from_utf8_unchecked(slice::from_raw_parts(data, size))
      })
    } else {
      "null".into()
    },
  );

  print!("{}", formatted);

  let mut context = unsafe { Box::from_raw(parser.context as *mut context::Context) };
  context.output.push_str(formatted.as_str());
  Box::into_raw(context);
  cleanup();
}

#[allow(dead_code)]
pub fn event(parser: &Parser, name: &str, position: usize, from: usize, size: usize) {
  append_output(
    parser,
    format!("\"pos\": {}, \"event\": \"{}\"", position, name),
    from,
    size,
  );
}

#[allow(dead_code)]
pub fn show_span(parser: &Parser, name: &str, from: usize, size: usize) {
  if name == "method" || name == "url" || name == "protocol" || name == "version" {
    let (data, cleanup) = extract_payload(parser, from, size);
    let mut context = unsafe { Box::from_raw(parser.context as *mut context::Context) };
    let value = unsafe { String::from_utf8_unchecked(slice::from_raw_parts(data, size).into()) };
    cleanup();

    match name {
      "method" => {
        context.method = value;
      }
      "url" => {
        context.url = value;
      }
      "protocol" => {
        context.protocol = value;
      }
      "version" => {
        context.version = value;
      }
      _ => {}
    }

    Box::into_raw(context);
  }

  event(parser, name, parser.position, from, size);
}
