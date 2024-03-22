#![allow(unused_variables, unreachable_code, unused_imports)]

#[path = "./context.rs"]
mod context;
mod output;

use std::{os::unix::process, slice, str};

use milo::{
  clear_offsets, error_code_string, flags, get_content_length, get_error_code, get_error_description_len,
  get_message_type, get_offsets_count, get_position, get_state, get_status, has_chunked_transfer_encoding,
  state_string, Offsets, Parser, DEBUG, MAX_OFFSETS_COUNT, OFFSET_BODY, OFFSET_CHUNK, OFFSET_CHUNK_EXTENSION_NAME,
  OFFSET_CHUNK_EXTENSION_VALUE, OFFSET_CHUNK_LENGTH, OFFSET_DATA, OFFSET_HEADERS, OFFSET_HEADER_NAME,
  OFFSET_HEADER_VALUE, OFFSET_MESSAGE_COMPLETE, OFFSET_MESSAGE_START, OFFSET_METHOD, OFFSET_PROTOCOL, OFFSET_REASON,
  OFFSET_STATUS, OFFSET_TRAILERS, OFFSET_TRAILER_NAME, OFFSET_TRAILER_VALUE, OFFSET_URL, OFFSET_VERSION, RESPONSE,
};

use self::output::extract_payload;

pub fn process_offsets(parser: &Parser) {
  let mut context = unsafe { Box::from_raw(parser.owner.get() as *mut context::Context) };
  let offsets = unsafe { Vec::from_raw_parts(parser.offsets, MAX_OFFSETS_COUNT * 3, MAX_OFFSETS_COUNT * 3) };

  let total = get_offsets_count(parser);
  clear_offsets(parser);

  for i in 0..total {
    let offset_from = offsets[i * 3 + 1];
    let offset_size = offsets[i * 3 + 2];

    let (data, cleanup) = output::extract_payload(parser, offset_from, offset_size);
    let value = if offset_size > 0 {
      Some(unsafe { String::from_utf8_unchecked(slice::from_raw_parts(data, offset_size).into()) })
    } else {
      None
    };

    cleanup();

    match offsets[i * 3] {
      OFFSET_MESSAGE_START => {
        output::event(parser, "offset.message_start", offset_from, offset_from, offset_size);
      }
      OFFSET_MESSAGE_COMPLETE => {
        output::event(parser, "offset.message_complete", offset_from, offset_from, offset_size);
      }
      OFFSET_METHOD => {
        output::event(parser, "offset.method", offset_from, offset_from, offset_size);
        context.method = value.unwrap();
      }
      OFFSET_URL => {
        output::event(parser, "offset.url", offset_from, offset_from, offset_size);
        context.url = value.unwrap();
      }
      OFFSET_PROTOCOL => {
        output::event(parser, "offset.protocol", offset_from, offset_from, offset_size);
        context.protocol = value.unwrap();
      }
      OFFSET_VERSION => {
        output::event(parser, "offset.version", offset_from, offset_from, offset_size);
        context.version = value.unwrap();
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
      OFFSET_HEADERS => {
        output::event(parser, "offset.headers", offset_from, offset_from, offset_size);
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
      OFFSET_CHUNK => {
        output::event(parser, "offset.chunk", offset_from, offset_from, offset_size);
      }
      OFFSET_DATA => {
        output::event(parser, "offset.data", offset_from, offset_from, offset_size);
      }
      OFFSET_BODY => {
        output::event(parser, "offset.body", offset_from, offset_from, offset_size);
      }
      OFFSET_TRAILER_NAME => {
        output::event(parser, "offset.trailer_name", offset_from, offset_from, offset_size);
      }
      OFFSET_TRAILER_VALUE => {
        output::event(parser, "offset.trailer_value", offset_from, offset_from, offset_size);
      }
      OFFSET_TRAILERS => {
        output::event(parser, "offset.trailers", offset_from, offset_from, offset_size);
      }
      _x => panic!("Unexpected offset with type {}", _x),
    };
  }

  // Remember to avoid dropping the memory
  offsets.into_raw_parts();
  Box::into_raw(context);
}

pub fn before_state_change(parser: &Parser, from: usize, size: usize) -> isize {
  output::append_output(
    parser,
    format!(
      "\"pos\": {}, \"event\": \"before_state_change\", \"current_state\": \"{}\"",
      get_position(parser),
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
      get_position(parser),
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
      get_position(parser),
      DEBUG,
    ),
    from,
    size,
  )
}

pub fn on_message_complete(parser: &Parser, from: usize, size: usize) -> isize {
  process_offsets(parser);
  output::event(parser, "complete", get_position(parser), from, size)
}

pub fn on_error(parser: &Parser, from: usize, size: usize) -> isize {
  unsafe {
    output::append_output(
      parser,
      format!(
        "\"pos\": {}, \"event\": {}, \"error_code={}, \"error_code_string\": \"{}\", reason=\"{}\"",
        get_position(parser),
        "error",
        get_error_code(parser),
        error_code_string(parser),
        str::from_utf8_unchecked(slice::from_raw_parts(
          parser.error_description.get(),
          get_error_description_len(parser)
        ))
      ),
      from,
      size,
    )
  }
}

pub fn on_finish(parser: &Parser, from: usize, size: usize) -> isize {
  output::event(parser, "finish", get_position(parser), from, size)
}

pub fn on_request(parser: &Parser, from: usize, size: usize) -> isize {
  output::event(parser, "request", get_position(parser), from, size)
}

pub fn on_response(parser: &Parser, from: usize, size: usize) -> isize {
  output::event(parser, "response", get_position(parser), from, size)
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
  process_offsets(parser);

  let context = unsafe { Box::from_raw(parser.owner.get() as *mut context::Context) };

  let position = get_position(parser);
  let chunked = has_chunked_transfer_encoding(parser);
  let content_length = get_content_length(parser);

  let method: String = context.method.clone();
  let url: String = context.url.clone();
  let protocol: String = context.protocol.clone();
  let version: String = context.version.clone();
  Box::into_raw(context);

  if get_message_type(parser) == RESPONSE {
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
          get_status(parser),
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
          get_status(parser),
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
          get_status(parser),
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
  output::event(parser, "upgrade", get_position(parser), from, size)
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
  process_offsets(parser);
  output::event(parser, "chunk", get_position(parser), from, size)
}

pub fn on_data(parser: &Parser, from: usize, size: usize) -> isize {
  process_offsets(parser);
  output::show_span(parser, "data", from, size)
}

pub fn on_body(parser: &Parser, from: usize, size: usize) -> isize {
  output::event(parser, "body", get_position(parser), from, size)
}

pub fn on_trailer_name(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "trailer_name", from, size)
}

pub fn on_trailer_value(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "trailer_value", from, size)
}

pub fn on_trailers(parser: &Parser, from: usize, size: usize) -> isize {
  process_offsets(parser);
  output::event(parser, "trailers", get_position(parser), from, size)
}
