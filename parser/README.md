# Milo Parser

This folder contains the milo main parser.

Milo is currently distributed as static library which is meant to be statically linked in a C++ executable (or anything that can link against a static library).

To build milo, simply run:

```bash
make
```

this will generate `libmilo.a` and `milo.h` in the `dist` folder.
These files can be copied in the source code to embed milo.

Alternatively, run:

```bash
make debug
```

To build a debug version of the static library which also enables the `before_state_change` and `after_state_change` callbacks for easier debugging.

## How to use and API

See the main [README.md](../README.md) in the parent folder.

## Run test suite

Rust tests are provided to ensure milo's integrity.

Run:

```bash
cargo test
```

## Verify compiled code and macro substitution

Once [cargo-expand](https://github.com/dtolnay/cargo-expand), is installed, run:

```bash
cargo expand > compiled.rs
```

This will generate a file which can be analyzed to see what is the final compiled code after all macros have been evaluated.
