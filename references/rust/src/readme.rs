use core::ffi::c_void;
use core::slice;

use milo::Parser;

fn main() {
  // Create the parser.
  let mut parser = Parser::new();

  // Prepare a message to parse.
  let message = String::from("HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc");
  parser.context = message.as_ptr() as *mut c_void;

  // Milo works using callbacks.
  //
  // All callbacks have the same signature, which characterizes the payload:
  //
  // p: The current parser.
  // from: The payload offset.
  // size: The payload length.
  //
  // The payload parameters above are relative to the last data sent to the parse
  // method.
  //
  // If the current callback has no payload, both values are set to 0.
  parser.callbacks.on_data = |p: &mut Parser, from: usize, size: usize| {
    let message =
      unsafe { std::str::from_utf8_unchecked(slice::from_raw_parts(p.context.add(from) as *const u8, size)) };

    // Do somethin cvdg with the informations.
    println!("Pos={} Body: {}", p.position, message);
  };

  // Now perform the main parsing using milo.parse. The method returns the number
  // of consumed characters.
  parser.parse(message.as_ptr(), message.len());
}
