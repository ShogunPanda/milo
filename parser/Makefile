TARGET:= $(shell rustc -vV | sed -n 's|host: ||p')
COMMON_OPTIONS := -Z unstable-options --target ${TARGET}
RELEASE_OPTIONS := -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort

.PHONY: all release debug clean

all: debug release

release: dist/milo.h target/dist/release/libmilo.a 
	cp target/dist/release/libmilo.a  dist/libmilo.a

debug: dist/milo.h target/dist/debug/libmilo.a
	cp target/dist/debug/libmilo.a dist/libmilo.a

clean: 
	rm -rf dist target

dist/milo.h:
	cbindgen --config cbindgen.toml --output dist/milo.h --clean
	node ./tools/export_version/main.mjs
	node ./tools/export_constants/main.mjs

target/dist/release/libmilo.a: src/lib.rs
	cargo build ${COMMON_OPTIONS} ${RELEASE_OPTIONS} --out-dir target/dist/release --release 

target/dist/debug/libmilo.a: src/lib.rs	
	cargo build ${COMMON_OPTIONS} --out-dir target/dist/debug
	