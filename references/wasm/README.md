# Milo WebAssembly Reference

This folder contains a simple reference Javascript file that uses milo (via WebAssembly).

In order to build it you need [make].

To build it simply execute:

```shell
$ make
```

This will compile milo as WebAssembly in `dist/debug` and `dist/release`.

The debug version will also show the parser state changes.

To execute the sample executable you can run:

```
node index.mjs [--debug|--release]
```

By default, it executes in release mode.

[make]: https://www.gnu.org/software/make/
