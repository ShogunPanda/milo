.PHONY: all clean

all: build/reference-debug build/reference-release

build/reference-debug: lib/milo.h lib/libmilo-debug.a
	clang++ -std=c++11 -D MILO_TEST_DEBUG=1 -I lib -o build/reference-debug lib/libmilo-debug.a src/main.cc

build/reference-release: lib/milo.h lib/libmilo-release.a
	clang++ -std=c++11 -I lib -o build/reference-release lib/libmilo-release.a src/main.cc

lib/milo.h:
	cd ../parser && rustup run nightly cbindgen --config cbindgen.toml --output ../reference/lib/milo.h

lib/libmilo-debug.a: ../parser/src/lib.rs
	cd ../parser && cargo +nightly build
	cp ../parser/target/debug/libmilo.a lib/libmilo-debug.a

lib/libmilo-release.a: ../parser/src/lib.rs
	cd ../parser && cargo +nightly build --release 
	cp ../parser/target/release/libmilo.a lib/libmilo-release.a

clean: 
	rm -rf build lib