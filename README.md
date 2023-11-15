# Milo

Milo is a fast and embeddable HTTP/1.1 parser written in [Rust][rust].

## Why?

The scope of Milo is to replace [llhttp] as [Node.js] main HTTP parser.

This project aims to:

- Make it maintainable and verificable using easy to read Rust code.
- Be performant by avoiding any unnecessary data copy.
- Be self-contained and dependency-free.

To see the rationale behind the replacement of llhttp, check Paolo's talk at [Vancouver's Node Collab Summit][vancouver-talk] in January 2023 ([slides][vancouver-slides]).

To see the initial disclosure of milo, check Paolo's talk at [Bilbao's Node Collab Summit][bilbao-talk] in September 2023 ([slides][bilbao-slides]).

## How it works?

Milo leverages Rust's [procedural macro], [syn] and [quote] crates to allow an easy definition of states and matchers for the parser.

See the [macros](./macros/README.md) internal crate for more information.

The data matching is possible thanks to power of the Rust's [match] statement applied to [data slices][match-slice].

The resulting parser is as simple state machine which copies the data in only one specific case, to automatically handle unconsumed portion of the input data.

In all other all cases, no data is copied and the memory footprint is very small as only 20 `intprt_t` fields can represent the entire parser state.

The performances are stunning, on a 2023 Apple M2 Max MacBook Pro it takes 500 nanosecond to parse a 1KB message.

## How to use it (WebAssembly)

Add the `dist/wasm` folder somewhere in your project. For instance, let's use: `deps/milo`.

Then create a sample source file:

```javascript
const { Parser } = require("./wasm");

// Create the parser
const parser = new Parser();
const message = Buffer.from("HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc");

// Milo works using callbacks.
// All callbacks have either no argument or a Uint8Array and its length.
// The callback MUST return 0 in case of success and non-zero in case of errors.
parser.setOnData((data, len) => {
  // Do something with the informations.
  console.log(
    `Pos=${parser.position} Body: ${Buffer.from(data).slice(0, len).toString()}`
  );

  // All good, let's return.
  return 0;
});

// This is the main method you invoke.
// It takes a Buffer and how many bytes to parse at most.
//
// It returns the number of consumed characters.
// Note that if not all characters are consumed, milo will automatically copies
// and prepends them in the next run of the function so there is no need to
// pass it again.
parser.parse(message, message.length);
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
milo = { path = "../parser" }
```

Create a sample source file:

```rust
use std::ffi::c_uchar;
use std::slice;
use std::str;

use milo::Parser;

fn main() {
  // Create the parser
  let parser = Parser::new();
  let message = "HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc";

  // Milo works using callbacks.
  // All callbacks have the same signature, but data can be eventually NULL (and
  // therefore size is 0). The callback should return 0 in case of success and
  // non-zero in case of errors.
  parser.callbacks.on_data = |p: &mut Parser, data: *const c_uchar, size: usize| -> isize {
    // Do something with the informations.
    println!("Pos={} Body: {}", p.position, unsafe {
      str::from_utf8_unchecked(slice::from_raw_parts(data, size))
    });

    // All good, let's return.
    0
  };

  // This is the main method you invoke.
  // It takes a data pointer and how many bytes to parse at most.
  //
  // It returns the number of consumed characters.
  // Note that if not all characters are consumed, milo will automatically copies
  // and prepends them in the next run of the function so there is no need to
  // pass it again.
  unsafe {
    parser.parse(message.as_ptr(), message.len());
  }
}
```

Finally build and execute it using `cargo`:

```bash
cargo run
# ... cargo build output ...
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

int main()
{
  // Create the parser
  milo::Parser *parser = milo::milo_create();
  const char *message = "HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc";

  /*
    Milo works using callbacks.
    All callbacks have the same signature, but data can be eventually NULL (and therefore size is 0).
    The callback should return 0 in case of success and non-zero in case of errors.
  */
  parser->callbacks.on_data = [](milo::Parser *p, const unsigned char *data, uintptr_t size) -> intptr_t
  {
    char *content = reinterpret_cast<char *>(malloc(sizeof(char) * 1000));
    // Rust internall uses unsigned chars for string, so we need to cast.
    strncpy(content, reinterpret_cast<const char *>(data), size);

    // Do something with the informations.
    printf("Pos=%lu Body: %s\n", p->position, content);
    free(content);

    // All good, let's return.
    return 0;
  };

  /*
    This is the main method you invoke.
    It takes a parser, a data pointer and how many bytes to parse at most.

    It returns the number of consumed characters.
    Note that if not all characters are consumed, milo will automatically copies and prepends them in
    the next run of the function so there is no need to pass it again.
  */
  milo::milo_parse(parser, reinterpret_cast<const unsigned char *>(message), strlen(message));

  // Remember not to leak memory. ;)
  milo::milo_destroy(parser);
}
```

And then you can compile using your preferred build system. For instance, let's try with [Clang]:

```bash
clang++ -std=c++11 -o example libmilo.a main.cc
./example
# Pos=38 Body: abc
```

### Build milo locally

If you want to build it locally, you need the following tools:

- [make]
- Rust toolchain - You can install it via [rustup].

Make sure you have the `nightly` toolchain installed locally:

```bash
rustup toolchain install nightly
```

After all the requirements are met, you can then run:

```bash
cd parser
make
```

The command above will produce the release file.
If you want the debug file (which also enables the `before_state_change` and `after_state_change` callbacks), run `make debug`.

## API

See the following files, according to the language you are using:

- [WebAssembly API](./docs/wasm.md)
- [Rust API](./docs/rust.md)
- [C++ API](./docs/cpp.md)

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
[nearform]: https://nearform.com
[llhttp]: https://github.com/nodejs/llhttp
[Node.js]: https://nodejs.org
[vancouver-talk]: https://youtube.com/watch?v=L-VONzXQ944
[vancouver-slides]: https://talks.cowtech.it/http-parser
[bilbao-talk]: http://localhost
[bilbao-slides]: https://talks.cowtech.it/milo/01
[isc]: https://choosealicense.com/licenses/isc
[procedural macro]: https://doc.rust-lang.org/reference/procedural-macros.html
[syn]: https://crates.io/crates/syn
[quote]: https://crates.io/crates/quote
[match]: https://doc.rust-lang.org/rust-by-example/flow_control/match.html
[match-slice]: https://doc.rust-lang.org/rust-by-example/flow_control/match/destructuring/destructure_slice.html
[make]: https://www.gnu.org/software/make/
[rust-up]: https://rustup.rs/
[Clang]: https://clang.llvm.org/
