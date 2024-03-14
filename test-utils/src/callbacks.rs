#![allow(unused_variables, unreachable_code, unused_imports)]

#[path = "./context.rs"]
mod context;
mod output;

use std::{slice, str};

use milo::{
  clear_offsets, error_code_string, flags, state_string, Offsets, Parser, DEBUG, MAX_OFFSETS_COUNT,
  OFFSET_CHUNK_EXTENSION_NAME, OFFSET_CHUNK_EXTENSION_VALUE, OFFSET_CHUNK_LENGTH, OFFSET_HEADER_NAME,
  OFFSET_HEADER_VALUE, OFFSET_METHOD, OFFSET_PROTOCOL, OFFSET_REASON, OFFSET_STATUS, OFFSET_TRAILER_NAME,
  OFFSET_TRAILER_VALUE, OFFSET_URL, OFFSET_VERSION, RESPONSE,
};

pub fn before_state_change(parser: &Parser, from: usize, size: usize) -> isize {
  output::append_output(
    parser,
    format!(
      "\"pos\": {}, \"event\": \"before_state_change\", \"current_state\": \"{}\"",
      parser.position.get(),
      state_string(parser)
    ),
    from,
    size,
  )
}

pub fn after_state_change(parser: &Parser, from: usize, size: usize) -> isize {
  output::append_output(
    parser,
    format!(
      "\"pos\": {}, \"event\": \"after_state_change\", \"current_state\": \"{}\"",
      parser.position.get(),
      state_string(parser)
    ),
    from,
    size,
  )
}

pub fn on_message_start(parser: &Parser, from: usize, size: usize) -> isize {
  output::append_output(
    parser,
    format!(
      "\"pos\": {}, \"event\": \"begin\", \"configuration\": {{ \"debug\": {} }}",
      parser.position.get(),
      DEBUG,
    ),
    from,
    size,
  )
}

pub fn on_message_complete(parser: &Parser, from: usize, size: usize) -> isize {
  output::event(parser, "complete", parser.position.get(), from, size)
}

pub fn on_error(parser: &Parser, from: usize, size: usize) -> isize {
  unsafe {
    output::append_output(
      parser,
      format!(
        "\"pos\": {}, \"event\": {}, \"error_code={}, \"error_code_string\": \"{}\", reason=\"{}\"",
        parser.position.get(),
        "error",
        parser.error_code.get(),
        error_code_string(parser),
        str::from_utf8_unchecked(slice::from_raw_parts(
          parser.error_description.get(),
          parser.error_description_len.get()
        ))
      ),
      from,
      size,
    )
  }
}

pub fn on_finish(parser: &Parser, from: usize, size: usize) -> isize {
  output::event(parser, "finish", parser.position.get(), from, size)
}

pub fn on_request(parser: &Parser, from: usize, size: usize) -> isize {
  output::event(parser, "request", parser.position.get(), from, size)
}

pub fn on_response(parser: &Parser, from: usize, size: usize) -> isize {
  output::event(parser, "response", parser.position.get(), from, size)
}

pub fn on_method(parser: &Parser, from: usize, size: usize) -> isize { output::show_span(parser, "method", from, size) }

pub fn on_url(parser: &Parser, from: usize, size: usize) -> isize { output::show_span(parser, "url", from, size) }

pub fn on_protocol(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "protocol", from, size)
}

pub fn on_version(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "version", from, size)
}

pub fn on_status(parser: &Parser, from: usize, size: usize) -> isize { output::show_span(parser, "status", from, size) }

pub fn on_reason(parser: &Parser, from: usize, size: usize) -> isize { output::show_span(parser, "reason", from, size) }

pub fn on_header_name(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "header_name", from, size)
}

pub fn on_header_value(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "header_value", from, size)
}

pub fn on_headers(parser: &Parser, from: usize, size: usize) -> isize {
  let context = unsafe { Box::from_raw(parser.owner.get() as *mut context::Context) };

  let position = parser.position.get();
  let chunked = parser.has_chunked_transfer_encoding.get();
  let content_length = parser.content_length.get();

  let mut method: String = "".into();
  let mut url: String = "".into();
  let mut protocol: String = "".into();
  let mut version: String = "".into();

  let offsets = unsafe { Vec::from_raw_parts(parser.offsets.get(), MAX_OFFSETS_COUNT, MAX_OFFSETS_COUNT) };

  let total = offsets[2];

  for i in 1..=total {
    let offset_from = offsets[i * 3 + 1];
    let offset_size = offsets[i * 3 + 2];

    let (data, cleanup) = output::extract_payload(parser, offset_from, offset_size);
    let value = unsafe { String::from_utf8_unchecked(slice::from_raw_parts(data, offset_size).into()) };
    cleanup();

    match offsets[i * 3] {
      OFFSET_METHOD => {
        output::event(parser, "offset.method", offset_from, offset_from, offset_size);
        method = value;
      }
      OFFSET_URL => {
        output::event(parser, "offset.url", offset_from, offset_from, offset_size);
        url = value;
      }
      OFFSET_PROTOCOL => {
        output::event(parser, "offset.protocol", offset_from, offset_from, offset_size);
        protocol = value;
      }
      OFFSET_VERSION => {
        output::event(parser, "offset.version", offset_from, offset_from, offset_size);
        version = value;
      }
      OFFSET_STATUS => {
        output::event(parser, "offset.status", offset_from, offset_from, offset_size);
      }
      OFFSET_REASON => {
        output::event(parser, "offset.reason", offset_from, offset_from, offset_size);
      }
      OFFSET_HEADER_NAME => {
        output::event(parser, "offset.header_name", offset_from, offset_from, offset_size);
      }
      OFFSET_HEADER_VALUE => {
        output::event(parser, "offset.header_value", offset_from, offset_from, offset_size);
      }
      OFFSET_CHUNK_LENGTH => {
        output::event(parser, "offset.chunk_length", offset_from, offset_from, offset_size);
      }
      OFFSET_CHUNK_EXTENSION_NAME => {
        output::event(
          parser,
          "offset.chunk_extensions_name",
          offset_from,
          offset_from,
          offset_size,
        );
      }
      OFFSET_CHUNK_EXTENSION_VALUE => {
        output::event(
          parser,
          "offset.chunk_extension_value",
          offset_from,
          offset_from,
          offset_size,
        );
      }
      _x => panic!("Unexpected offset with type {}", _x),
    };
  }

  // Remember to avoid dropping the memory
  offsets.into_raw_parts();

  Box::into_raw(context);

  if parser.message_type.get() == RESPONSE {
    let heading = format!(
      "\"pos\": {}, \"event\": {}, \"type\": \"response\", ",
      position,
      output::format_event("headers")
    );

    if chunked {
      output::append_output(
        parser,
        format!(
          "{}\"status\": {}, \"protocol\": \"{}\", \"version\": \"{}\", \"body\": \"chunked\"",
          heading,
          parser.status.get(),
          protocol,
          version,
        ),
        from,
        size,
      )
    } else if content_length > 0 {
      output::append_output(
        parser,
        format!(
          "{}\"status\": {}, \"protocol\": \"{}\", \"version\": \"{}\", \"body\": {}",
          heading,
          parser.status.get(),
          protocol,
          version,
          content_length
        ),
        from,
        size,
      )
    } else {
      output::append_output(
        parser,
        format!(
          "{}\"status\": {}, \"protocol\": \"{}\", \"version\": \"{}\", \"body\": null",
          heading,
          parser.status.get(),
          protocol,
          version,
        ),
        from,
        size,
      )
    }
  } else {
    let heading = format!(
      "\"pos\": {}, \"event\": {}, \"type\": \"request\", ",
      position,
      output::format_event("headers")
    );

    if chunked {
      output::append_output(
        parser,
        format!(
          "{}\"method\": \"{}\", \"url\": \"{}\", \"protocol\": \"{}\", \"version\": \"{}\", \"body\": \"chunked\"",
          heading, method, url, protocol, version,
        ),
        from,
        size,
      )
    } else if content_length > 0 {
      output::append_output(
        parser,
        format!(
          "{}\"method\": \"{}\", \"url\": \"{}\", \"protocol\": \"{}\", \"version\": \"{}\", \"body\": {}",
          heading, method, url, protocol, version, content_length
        ),
        from,
        size,
      )
    } else {
      output::append_output(
        parser,
        format!(
          "{}\"method\": \"{}\", \"url\": \"{}\", \"protocol\": \"{}\", \"version\": \"{}\", \"body\": null",
          heading, method, url, protocol, version,
        ),
        from,
        size,
      )
    }
  }
}

pub fn on_upgrade(parser: &Parser, from: usize, size: usize) -> isize {
  output::event(parser, "upgrade", parser.position.get(), from, size)
}

pub fn on_chunk_length(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "chunk_length", from, size)
}

pub fn on_chunk_extension_name(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "chunk_extensions_name", from, size)
}

pub fn on_chunk_extension_value(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "chunk_extension_value", from, size)
}

pub fn on_chunk(parser: &Parser, from: usize, size: usize) -> isize {
  let offsets = unsafe { Vec::from_raw_parts(parser.offsets.get(), MAX_OFFSETS_COUNT, MAX_OFFSETS_COUNT) };
  let total = offsets[2];

  for i in 1..=total {
    let offset_from = offsets[i * 3 + 1];
    let offset_size = offsets[i * 3 + 2];

    match offsets[i * 3] {
      OFFSET_CHUNK_LENGTH => {
        output::event(parser, "offset.chunk_length", offset_from, offset_from, offset_size);
      }
      OFFSET_CHUNK_EXTENSION_NAME => {
        output::event(
          parser,
          "offset.chunk_extensions_name",
          offset_from,
          offset_from,
          offset_size,
        );
      }
      OFFSET_CHUNK_EXTENSION_VALUE => {
        output::event(
          parser,
          "offset.chunk_extension_value",
          offset_from,
          offset_from,
          offset_size,
        );
      }
      _x => {}
    };
  }

  offsets.into_raw_parts();

  clear_offsets(parser);

  output::event(parser, "chunk", parser.position.get(), from, size)
}

pub fn on_data(parser: &Parser, from: usize, size: usize) -> isize { output::show_span(parser, "data", from, size) }

pub fn on_body(parser: &Parser, from: usize, size: usize) -> isize {
  output::event(parser, "body", parser.position.get(), from, size)
}

pub fn on_trailer_name(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "trailer_name", from, size)
}

pub fn on_trailer_value(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "trailer_value", from, size)
}

pub fn on_trailers(parser: &Parser, from: usize, size: usize) -> isize {
  let offsets = unsafe { Vec::from_raw_parts(parser.offsets.get(), MAX_OFFSETS_COUNT, MAX_OFFSETS_COUNT) };
  let total = offsets[2];

  for i in 1..=total {
    let offset_from = offsets[i * 3 + 1];
    let offset_size = offsets[i * 3 + 2];

    match offsets[i * 3] {
      OFFSET_TRAILER_NAME => {
        output::event(parser, "offset.trailer_name", offset_from, offset_from, offset_size);
      }
      OFFSET_TRAILER_VALUE => {
        output::event(parser, "offset.trailer_value", offset_from, offset_from, offset_size);
      }
      _x => {}
    };
  }

  offsets.into_raw_parts();

  output::event(parser, "trailers", parser.position.get(), from, size)
}
