# Milo WebAssembly Reference

This folder contains a simple reference Javascript file that uses milo (via WebAssembly).

In order to build it you need [cargo-make].

To build it simply execute:

```bash
cargo make
```

This will generate two executables in the `dist` directory `reference-debug` and `reference-release`.

The debug version will also show the parser state changes.

[cargo-make]: https://sagiegurari.github.io/cargo-make/
