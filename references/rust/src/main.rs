use std::os::raw::c_void;

use milo::Parser;
use milo_test_utils::{callbacks, context::Context, parse};

fn main() {
  let parser = Parser::new();
  let context = Box::new(Context::new());
  parser.owner.set(Box::into_raw(context) as *mut c_void);

  let request1 = "GET / HTTP/1.1\r\n\r\n";
  let request2 = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: x-trailer\r\n\r\nc;need=love\r\nhello \
                  world!\r\n0\r\nX-Trailer: value\r\n\r\n";

  parser.callbacks.before_state_change.set(callbacks::before_state_change);
  parser.callbacks.after_state_change.set(callbacks::after_state_change);
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

  let mut consumed = parse(&parser, request1);

  println!(
    "{{ \"pos\": {}, \"consumed\": {}, \"state\": \"{}\" }}",
    parser.position.get(),
    consumed,
    parser.state_string()
  );

  println!("\n------------------------------------------------------------------------------------------\n");

  consumed = parse(&parser, request2);
  println!(
    "{{ \"pos\": {}, \"consumed\": {}, \"state\": \"{}\" }}",
    parser.position.get(),
    consumed,
    parser.state_string()
  );
}
