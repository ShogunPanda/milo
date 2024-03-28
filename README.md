# @perseveranza-pets/milo

Milo is a fast and embeddable HTTP/1.1 parser written in [Rust][rust].

## How to use it (JavaScript via WebAssembly)

Install it from npm:

```
npm install @perseveranza-pets/milo
```

Then create a sample source file:

```javascript
const milo = require('@perseveranza-pets/milo')

// Prepare a message to parse.
const message = Buffer.from('HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc')

// Allocate a memory in the WebAssembly space. This speeds up data copying to the WebAssembly layer.
const ptr = milo.alloc(message.length)

// Create a buffer we can use normally.
const buffer = Buffer.from(milo.memory.buffer, ptr, message.length)

// Create the parser.
const parser = milo.create()

/*
  Milo works using callbacks.

  All callbacks have the same signature, which characterizes the payload:
  
    * The current parent
    * from: The payload offset.
    * size: The payload length.
    
  The payload parameters above are relative to the last data sent to the milo.parse method.

  If the current callback has no payload, both values are set to 0.
*/
milo.setOnData(parser, (p, from, size) => {
  console.log(`Pos=${milo.getPosition(p)} Body: ${message.slice(from, from + size).toString()}`)
})

// Now perform the main parsing using milo.parse. The method returns the number of consumed characters.
buffer.set(Buffer.from(message), 0)
const consumed = milo.parse(parser, ptr, message.length)

// Cleanup used resources.
milo.destroy(parser)
milo.dealloc(ptr, message.length)
```

Finally build and execute it using `node`:

```bash
node index.js
# Pos=38 Body: abc
```

## How to use it (Rust)

Add `milo` to your `Cargo.toml`:

```toml
[package]
name = "milo-example"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
milo = "0.1.0"
```

Create a sample source file:

```rust
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

```

Finally build and execute it using `cargo`:

```bash
cargo run
# Pos=38 Body: abc
```

## How to use it (C++)

First, let's download Milo release from GitHub.

You will need a static library file (for Linux/Unix/MacOS is `libmilo.a`) and a header file (`milo.h`).

Create a sample source file:

```cpp
#include "milo.h"
#include "stdio.h"
#include "string.h"

int main() {
  // Create the parser.
  milo::Parser* parser = milo::milo_create();

  // Prepare a message to parse.
  const char* message = "HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc";

  parser->context = (char*) message;

  /*
    Milo works using callbacks.

    All callbacks have the same signature, which characterizes the payload:

      * p: The current parser.
      * at: The payload offset.
      * len: The payload length.

    The payload parameters above are relative to the last data sent to the milo_parse method.

    If the current callback has no payload, both values are set to 0.
  */
  parser->callbacks.on_data = [](milo::Parser* p, uintptr_t from, uintptr_t size) {
    char* payload = reinterpret_cast<char*>(malloc(sizeof(char) * size));
    strncpy(payload, reinterpret_cast<const char*>(p->context) + from, size);

    printf("Pos=%lu Body: %s\n", p->position, payload);
    free(payload);
  };

  // Now perform the main parsing using milo.parse. The method returns the number of consumed characters.
  milo::milo_parse(parser, reinterpret_cast<const unsigned char*>(message), strlen(message));

  // Cleanup used resources.
  milo::milo_destroy(parser);
}
```

And then you can compile using your preferred build system. For instance, let's try with [Clang]:

```bash
clang++ -std=c++11 -o example libmilo.a main.cc
./example
# Pos=38 Body: abc
```

### Build milo (WebAssembly and C++) locally

If you want to build it locally, you need the following tools:

- [cargo-make][cargo-make]
- Rust toolchain - You can install it via [rustup].

Make sure you have the `nightly` toolchain installed locally:

```bash
rustup toolchain install nightly
```

After all the requirements are met, you can then run:

```bash
cd parser
makers
```

The command above will produce debug and release builds for each language in the `dist` folder.

The debug build will also enables the `before_state_change` and `after_state_change` callbacks and it's more verbose in case of WebAssembly errors.

## API

See the following files, according to the language you are using:

- [JavaScript via WebAssembly API](./docs/js.md)
- [Rust API](./docs/rust.md)
- [C++ API](./docs/cpp.md)

## How it works?

Milo leverages Rust's [procedural macro], [syn] and [quote] crates to allow an easy definition of states and matchers for the parser.

See the [macros](./macros/README.md) internal crate for more information.

The data matching is possible thanks to power of the Rust's [match] statement applied to [data slices][match-slice].

The resulting parser is as simple state machine which copies the data in only one (optional) specific case: to automatically handle unconsumed portion of the input data.

In all other all cases, no data is copied and the memory footprint is very small as only 30 `bool`, `uintprt_t` or `uint64_t` fields can represent the entire parser state.

## Why?

The scope of Milo is to replace [llhttp] as [Node.js] main HTTP parser.

This project aims to:

- Make it maintainable and verificable using easy to read Rust code.
- Be performant by avoiding any unnecessary data copy.
- Be self-contained and dependency-free.

To see the rationale behind the replacement of llhttp, check Paolo's talk at [Vancouver's Node Collab Summit][vancouver-talk] in January 2023 ([slides][vancouver-slides]).

To see the initial disclosure of milo, check Paolo's talk at [NodeConf EU 2023][nodeconf-talk] in November 2023 ([slides][slides]).

## Sponsored by

[![NearForm](https://raw.githubusercontent.com/ShogunPanda/milo/main/docs/nearform.jpg)][nearform]

## Contributing to milo

- Check out the latest master to make sure the feature hasn't been implemented or the bug hasn't been fixed yet.
- Check out the issue tracker to make sure someone already hasn't requested it and/or contributed it.
- Fork the project.
- Start a feature/bugfix branch.
- Commit and push until you are happy with your contribution.
- Make sure to add tests for it. This is important so I don't break it in a future version unintentionally.

## Copyright

Copyright (C) 2023 and above Paolo Insogna (paolo@cowtech.it) and NearForm (https://nearform.com).

Licensed under the ISC license, which can be found at https://choosealicense.com/licenses/isc or in the [LICENSE.md](./LICENSE.md) file.

[rust]: https://www.rust-lang.org/
[webassembly]: https://webassembly.org/
[nearform]: https://nearform.com
[llhttp]: https://github.com/nodejs/llhttp
[Node.js]: https://nodejs.org
[vancouver-talk]: https://youtube.com/watch?v=L-VONzXQ944
[vancouver-slides]: https://talks.cowtech.it/http-parser
[nodeconf-talk]: https://youtube.com/watch?v=dcHbAeO_ccY
[slides]: https://talks.paoloinsogna.dev/milo
[isc]: https://choosealicense.com/licenses/isc
[procedural macro]: https://doc.rust-lang.org/reference/procedural-macros.html
[syn]: https://crates.io/crates/syn
[quote]: https://crates.io/crates/quote
[match]: https://doc.rust-lang.org/rust-by-example/flow_control/match.html
[match-slice]: https://doc.rust-lang.org/rust-by-example/flow_control/match/destructuring/destructure_slice.html
[cargo-make]: https://github.com/sagiegurari/cargo-make
[rust-up]: https://rustup.rs/
[Clang]: https://clang.llvm.org/
