use std::os::raw::c_void;

use milo::Parser;
use milo_test_utils::{callbacks, context::Context, parse};

fn main() {
  let mut parser = Parser::new();
  let context = Box::new(Context::new());
  parser.context = Box::into_raw(context) as *mut c_void;

  let request1 = "GET / HTTP/1.1\r\n\r\n";
  let request2 = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: x-trailer\r\n\r\nc;need=love\r\nhello \
                  world!\r\n0\r\nX-Trailer: value\r\n\r\n";

  parser.callbacks.on_state_change = callbacks::on_state_change;
  parser.callbacks.on_error = callbacks::on_error;
  parser.callbacks.on_finish = callbacks::on_finish;
  parser.callbacks.on_request = callbacks::on_request;
  parser.callbacks.on_response = callbacks::on_response;
  parser.callbacks.on_message_start = callbacks::on_message_start;
  parser.callbacks.on_message_complete = callbacks::on_message_complete;
  parser.callbacks.on_method = callbacks::on_method;
  parser.callbacks.on_url = callbacks::on_url;
  parser.callbacks.on_protocol = callbacks::on_protocol;
  parser.callbacks.on_version = callbacks::on_version;
  parser.callbacks.on_status = callbacks::on_status;
  parser.callbacks.on_reason = callbacks::on_reason;
  parser.callbacks.on_header_name = callbacks::on_header_name;
  parser.callbacks.on_header_value = callbacks::on_header_value;
  parser.callbacks.on_headers = callbacks::on_headers;
  parser.callbacks.on_upgrade = callbacks::on_upgrade;
  parser.callbacks.on_chunk_length = callbacks::on_chunk_length;
  parser.callbacks.on_chunk_extension_name = callbacks::on_chunk_extension_name;
  parser.callbacks.on_chunk_extension_value = callbacks::on_chunk_extension_value;
  parser.callbacks.on_chunk = callbacks::on_chunk;
  parser.callbacks.on_body = callbacks::on_body;
  parser.callbacks.on_data = callbacks::on_data;
  parser.callbacks.on_trailer_name = callbacks::on_trailer_name;
  parser.callbacks.on_trailer_value = callbacks::on_trailer_value;
  parser.callbacks.on_trailers = callbacks::on_trailers;

  let mut consumed = parse(&mut parser, request1);

  println!(
    "{{ \"pos\": {}, \"consumed\": {}, \"state\": \"{}\" }}",
    parser.position,
    consumed,
    parser.state_str(),
  );

  println!("\n------------------------------------------------------------------------------------------\n");

  consumed = parse(&mut parser, request2);
  println!(
    "{{ \"pos\": {}, \"consumed\": {}, \"state\": \"{}\" }}",
    parser.position,
    consumed,
    parser.state_str(),
  );
}
